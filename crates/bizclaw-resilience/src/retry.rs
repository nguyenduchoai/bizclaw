//! Retry with Exponential Backoff

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
            jitter_factor: 0.1,
        }
    }
}

impl RetryConfig {
    pub fn builder() -> RetryConfigBuilder {
        RetryConfigBuilder::new()
    }
}

pub struct RetryConfigBuilder {
    config: RetryConfig,
}

impl Default for RetryConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: RetryConfig::default(),
        }
    }

    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.config.max_attempts = attempts;
        self
    }

    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.config.initial_delay = delay;
        self
    }

    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.config.max_delay = delay;
        self
    }

    pub fn backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.config.backoff_multiplier = multiplier;
        self
    }

    pub fn with_jitter(mut self) -> Self {
        self.config.jitter = true;
        self
    }

    pub fn without_jitter(mut self) -> Self {
        self.config.jitter = false;
        self
    }

    pub fn build(self) -> RetryConfig {
        self.config
    }
}

pub async fn retry<F, Fut, T, E>(config: RetryConfig, operation: F) -> Result<T, RetryError<E>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error,
{
    let mut attempts = 0;
    let mut delay = config.initial_delay;
    let mut operation = operation;

    loop {
        attempts += 1;

        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempts >= config.max_attempts {
                    return Err(RetryError::Exhausted {
                        attempts,
                        last_error: e,
                    });
                }

                let wait_time = calculate_delay(&config, attempts);

                if config.jitter {
                    let jitter = wait_time.as_secs_f64() * config.jitter_factor;
                    let jitter_duration =
                        Duration::from_secs_f64(jitter * (rand_simple() * 2.0 - 1.0));
                    let final_delay = wait_time.saturating_add(jitter_duration);
                    sleep(final_delay).await;
                } else {
                    sleep(wait_time).await;
                }

                delay = min_duration(
                    delay.mul_f64(config.backoff_multiplier),
                    config.max_delay,
                );
            }
        }
    }
}

fn calculate_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let base_delay = config.initial_delay.as_secs_f64() * config.backoff_multiplier.powi(attempt as i32 - 1);
    min_duration(Duration::from_secs_f64(base_delay), config.max_delay)
}

fn min_duration(a: Duration, b: Duration) -> Duration {
    if a < b {
        a
    } else {
        b
    }
}

fn rand_simple() -> f64 {
    use std::time::Instant;
    let seed = Instant::now().elapsed().as_nanos() as u64;
    ((seed ^ (seed >> 17) ^ (seed << 5)) % 1000) as f64 / 1000.0
}

#[derive(Debug, thiserror::Error)]
pub enum RetryError<E> {
    #[error("Retry attempts exhausted after {attempts} attempts")]
    Exhausted { attempts: u32, last_error: E },
}



#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_succeeds_on_second_attempt() {
        let mut attempts = 0;

        let result = retry(RetryConfig::default(), || async {
            attempts += 1;
            if attempts < 2 {
                Err::<i32, _>(std::io::Error::new(std::io::ErrorKind::Other, "fail"))
            } else {
                Ok(42)
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn test_retry_exhausts_after_max_attempts() {
        let result = retry(
            RetryConfig {
                max_attempts: 2,
                initial_delay: Duration::from_millis(10),
                ..Default::default()
            },
            || async { Err::<i32, _>(std::io::Error::new(std::io::ErrorKind::Other, "fail")) },
        )
        .await;

        assert!(matches!(
            result,
            Err(RetryError::Exhausted { attempts: 2, .. })
        ));
    }
}
