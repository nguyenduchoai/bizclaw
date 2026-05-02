//! # BizClaw Resilience
//!
//! Production resilience patterns: rate limiting, circuit breaker, retry with backoff.
//!
//! ## Features
//!
//! ### Rate Limiter
//! - Token bucket algorithm
//! - Sliding window algorithm
//! - Fixed window algorithm
//! - Multi-tenant support
//!
//! ### Circuit Breaker
//! - Closed/Open/Half-Open states
//! - Configurable failure thresholds
//! - Automatic recovery
//! - Multi-service support
//!
//! ### Retry
//! - Exponential backoff
//! - Jitter support
//! - Configurable delays
//! - Future-based API
//!
//! ## Usage
//!
//! ```rust
//! use bizclaw_resilience::{RateLimiter, CircuitBreaker, retry, RetryConfig};
//! use std::time::Duration;
//!
//! // Rate limiter
//! let limiter = RateLimiter::new(
//!     RateLimiterConfig {
//!         requests_per_second: 100,
//!         burst_size: 200,
//!         window_seconds: 60,
//!     },
//!     RateLimitAlgorithm::SlidingWindow,
//! );
//!
//! if let Ok(token) = limiter.try_acquire() {
//!     // proceed with request
//! }
//!
//! // Circuit breaker
//! let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
//! let result = breaker.execute(|| async { call_external_service().await }).await;
//!
//! // Retry with backoff
//! let result = retry(RetryConfig::default(), || async {
//!     call_unreliable_service().await
//! }).await;
//! ```

pub mod circuit_breaker;
pub mod rate_limiter;
pub mod retry;

pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerMetrics, CircuitState,
    MultiServiceCircuitBreaker,
};
pub use rate_limiter::{
    MultiTenantRateLimiter, RateLimiter, RateLimiterConfig, RateLimitAlgorithm, RateLimitExceeded,
    RateLimitToken,
};
pub use retry::{retry, RetryConfig, RetryConfigBuilder, RetryError};
