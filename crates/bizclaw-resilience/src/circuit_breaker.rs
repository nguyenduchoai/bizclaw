//! Circuit Breaker - Prevents cascading failures

use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
        }
    }
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
}

struct CircuitBreakerState {
    circuit_state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState {
                circuit_state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure: None,
            })),
        }
    }

    pub fn state(&self) -> CircuitState {
        let state = self.state.read();
        match state.circuit_state {
            CircuitState::Closed => CircuitState::Closed,
            CircuitState::Open => {
                if let Some(last_failure) = state.last_failure {
                    if last_failure.elapsed() >= self.config.timeout {
                        CircuitState::HalfOpen
                    } else {
                        CircuitState::Open
                    }
                } else {
                    CircuitState::HalfOpen
                }
            }
            CircuitState::HalfOpen => CircuitState::HalfOpen,
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self.state(), CircuitState::Closed | CircuitState::HalfOpen)
    }

    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        if !self.is_available() {
            return Err(CircuitBreakerError::Open {
                retry_after: self.config.timeout,
            });
        }

        match operation().await {
            Ok(value) => {
                self.on_success();
                Ok(value)
            }
            Err(e) => {
                self.on_failure();
                Err(CircuitBreakerError::ExecutionFailed(Box::new(e)))
            }
        }
    }

    pub fn execute_sync<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        if !self.is_available() {
            return Err(CircuitBreakerError::Open {
                retry_after: self.config.timeout,
            });
        }

        match operation() {
            Ok(value) => {
                self.on_success();
                Ok(value)
            }
            Err(e) => {
                self.on_failure();
                Err(CircuitBreakerError::ExecutionFailed(Box::new(e)))
            }
        }
    }

    fn on_success(&self) {
        let mut state = self.state.write();
        match state.circuit_state {
            CircuitState::Closed => {
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.config.success_threshold {
                    state.circuit_state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    fn on_failure(&self) {
        let mut state = self.state.write();
        state.failure_count += 1;
        state.last_failure = Some(Instant::now());

        match state.circuit_state {
            CircuitState::Closed => {
                if state.failure_count >= self.config.failure_threshold {
                    state.circuit_state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                state.circuit_state = CircuitState::Open;
                state.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    pub fn reset(&self) {
        let mut state = self.state.write();
        state.circuit_state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure = None;
    }

    pub fn metrics(&self) -> CircuitBreakerMetrics {
        let state = self.state.read();
        CircuitBreakerMetrics {
            state: state.circuit_state,
            failure_count: state.failure_count,
            success_count: state.success_count,
            last_failure: state.last_failure,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure: Option<Instant>,
}

#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is open. Retry after {retry_after:?}")]
    Open { retry_after: Duration },

    #[error("Operation failed: {0}")]
    ExecutionFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub struct MultiServiceCircuitBreaker {
    breakers: dashmap::DashMap<String, Arc<CircuitBreaker>>,
    default_config: CircuitBreakerConfig,
}

impl MultiServiceCircuitBreaker {
    pub fn new(default_config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: dashmap::DashMap::new(),
            default_config,
        }
    }

    pub fn get_or_create(&self, service_name: &str) -> Arc<CircuitBreaker> {
        self.breakers
            .entry(service_name.to_string())
            .or_insert_with(|| Arc::new(CircuitBreaker::new(self.default_config.clone())))
            .clone()
    }

    pub fn is_available(&self, service_name: &str) -> bool {
        self.breakers
            .get(service_name)
            .map(|b| b.is_available())
            .unwrap_or(true)
    }

    pub fn reset(&self, service_name: &str) {
        if let Some(breaker) = self.breakers.get(service_name) {
            breaker.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            half_open_max_calls: 2,
        };

        let breaker = CircuitBreaker::new(config);

        for _ in 0..3 {
            let result = breaker
                .execute(|| async { Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test")) })
                .await;
            assert!(result.is_err());
        }

        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
            half_open_max_calls: 1,
        };

        let breaker = CircuitBreaker::new(config);

        breaker
            .execute(|| async { Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test")) })
            .await
            .unwrap_err();

        assert_eq!(breaker.state(), CircuitState::Open);

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }
}
