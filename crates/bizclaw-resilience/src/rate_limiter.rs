//! Rate Limiter - Token bucket and sliding window algorithms

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub window_seconds: u64,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_size: 200,
            window_seconds: 60,
        }
    }
}

pub enum RateLimitAlgorithm {
    TokenBucket,
    SlidingWindow,
    FixedWindow,
}

pub struct RateLimiter {
    config: RateLimiterConfig,
    algorithm: RateLimitAlgorithm,
    state: Arc<RwLock<RateLimiterState>>,
}

struct RateLimiterState {
    tokens: f64,
    last_refill: Instant,
    requests: VecDeque<Instant>,
}

impl RateLimiter {
    pub fn new(config: RateLimiterConfig, algorithm: RateLimitAlgorithm) -> Self {
        Self {
            config,
            algorithm,
            state: Arc::new(RwLock::new(RateLimiterState {
                tokens: config.burst_size as f64,
                last_refill: Instant::now(),
                requests: VecDeque::new(),
            })),
        }
    }

    pub fn try_acquire(&self) -> Result<RateLimitToken, RateLimitExceeded> {
        match self.algorithm {
            RateLimitAlgorithm::TokenBucket => self.try_acquire_token_bucket(),
            RateLimitAlgorithm::SlidingWindow => self.try_acquire_sliding_window(),
            RateLimitAlgorithm::FixedWindow => self.try_acquire_fixed_window(),
        }
    }

    fn try_acquire_token_bucket(&self) -> Result<RateLimitToken, RateLimitExceeded> {
        let mut state = self.state.write();
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill);

        let tokens_to_add = (elapsed.as_secs_f64() * self.config.requests_per_second as f64).floor();
        state.tokens = (state.tokens + tokens_to_add).min(self.config.burst_size as f64);
        state.last_refill = now;

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            Ok(RateLimitToken {
                available_at: None,
                remaining: state.tokens as u32,
            })
        } else {
            let wait_time = Duration::from_secs_f64(1.0 / self.config.requests_per_second as f64);
            Err(RateLimitExceeded {
                retry_after: wait_time,
                remaining: 0,
            })
        }
    }

    fn try_acquire_sliding_window(&self) -> Result<RateLimitToken, RateLimitExceeded> {
        let mut state = self.state.write();
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_seconds);

        while state.requests.front().map(|t| now.duration_since(*t) > window).unwrap_or(false) {
            state.requests.pop_front();
        }

        if state.requests.len() < self.config.requests_per_second as usize {
            state.requests.push_back(now);
            Ok(RateLimitToken {
                available_at: None,
                remaining: (self.config.requests_per_second - state.requests.len() as u32),
            })
        } else {
            let oldest = state.requests.front().copied();
            let retry_after = oldest
                .map(|t| t + window - now)
                .unwrap_or(Duration::from_secs(1));

            Err(RateLimitExceeded {
                retry_after,
                remaining: 0,
            })
        }
    }

    fn try_acquire_fixed_window(&self) -> Result<RateLimitToken, RateLimitExceeded> {
        let mut state = self.state.write();
        let now = Instant::now();

        while state.requests.front().map(|t| now.duration_since(*t) > Duration::from_secs(self.config.window_seconds)).unwrap_or(false) {
            state.requests.clear();
        }

        if state.requests.len() < self.config.burst_size as usize {
            state.requests.push_back(now);
            Ok(RateLimitToken {
                available_at: None,
                remaining: self.config.burst_size - state.requests.len() as u32,
            })
        } else {
            Err(RateLimitExceeded {
                retry_after: Duration::from_secs(self.config.window_seconds),
                remaining: 0,
            })
        }
    }

    pub fn available(&self) -> u32 {
        let state = self.state.read();
        match self.algorithm {
            RateLimitAlgorithm::TokenBucket => state.tokens as u32,
            RateLimitAlgorithm::SlidingWindow | RateLimitAlgorithm::FixedWindow => {
                (self.config.requests_per_second - state.requests.len() as u32).max(0)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitToken {
    pub available_at: Option<Instant>,
    pub remaining: u32,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Rate limit exceeded. Retry after {retry_after:?}")]
pub struct RateLimitExceeded {
    pub retry_after: Duration,
    pub remaining: u32,
}

pub struct MultiTenantRateLimiter {
    limiters: dashmap::DashMap<String, RateLimiter>,
    default_config: RateLimiterConfig,
}

impl MultiTenantRateLimiter {
    pub fn new(default_config: RateLimiterConfig) -> Self {
        Self {
            limiters: dashmap::DashMap::new(),
            default_config,
        }
    }

    pub fn get_or_create(&self, tenant_id: &str) -> Arc<RateLimiter> {
        self.limiters
            .entry(tenant_id.to_string())
            .or_insert_with(|| RateLimiter::new(self.default_config.clone(), RateLimitAlgorithm::SlidingWindow))
            .clone()
    }

    pub fn try_acquire(&self, tenant_id: &str) -> Result<RateLimitToken, RateLimitExceeded> {
        let limiter = self.get_or_create(tenant_id);
        limiter.try_acquire()
    }

    pub fn remove(&self, tenant_id: &str) {
        self.limiters.remove(tenant_id);
    }

    pub fn len(&self) -> usize {
        self.limiters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_bucket_rate_limiter() {
        let config = RateLimiterConfig {
            requests_per_second: 10,
            burst_size: 5,
            window_seconds: 1,
        };

        let limiter = RateLimiter::new(config, RateLimitAlgorithm::TokenBucket);

        for i in 0..5 {
            assert!(limiter.try_acquire().is_ok(), "Request {} should succeed", i);
        }

        assert!(limiter.try_acquire().is_err());
    }

    #[tokio::test]
    async fn test_sliding_window_rate_limiter() {
        let config = RateLimiterConfig {
            requests_per_second: 10,
            burst_size: 10,
            window_seconds: 1,
        };

        let limiter = RateLimiter::new(config, RateLimitAlgorithm::SlidingWindow);

        for i in 0..10 {
            assert!(limiter.try_acquire().is_ok(), "Request {} should succeed", i);
        }

        assert!(limiter.try_acquire().is_err());
    }

    #[tokio::test]
    async fn test_multi_tenant_rate_limiter() {
        let config = RateLimiterConfig::default();
        let limiter = MultiTenantRateLimiter::new(config);

        assert!(limiter.try_acquire("tenant1").is_ok());
        assert!(limiter.try_acquire("tenant2").is_ok());
        assert!(limiter.try_acquire("tenant1").is_ok());

        assert_eq!(limiter.len(), 2);
    }
}
