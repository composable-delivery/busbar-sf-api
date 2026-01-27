//! Retry policy with exponential backoff and jitter.

use std::time::Duration;
use rand::Rng;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Initial delay before first retry.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Backoff strategy to use.
    pub backoff: BackoffStrategy,
    /// Whether to respect Retry-After headers.
    pub respect_retry_after: bool,
    /// Maximum time to wait from Retry-After header.
    pub max_retry_after: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff: BackoffStrategy::ExponentialWithJitter { factor: 2.0 },
            respect_retry_after: true,
            max_retry_after: Duration::from_secs(60),
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with the given max attempts.
    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Create a new retry config with the given initial delay.
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Create a new retry config with the given max delay.
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Create a new retry config with the given backoff strategy.
    pub fn with_backoff(mut self, backoff: BackoffStrategy) -> Self {
        self.backoff = backoff;
        self
    }

    /// Disable retries.
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 0,
            ..Default::default()
        }
    }

    /// Aggressive retry config for important operations.
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            backoff: BackoffStrategy::ExponentialWithJitter { factor: 2.0 },
            respect_retry_after: true,
            max_retry_after: Duration::from_secs(120),
        }
    }
}

/// Backoff strategy for determining retry delays.
#[derive(Debug, Clone, Copy)]
pub enum BackoffStrategy {
    /// Constant delay between retries.
    Constant,
    /// Linear increase in delay (delay * attempt).
    Linear,
    /// Exponential increase in delay (delay * factor^attempt).
    Exponential { factor: f64 },
    /// Exponential with random jitter to avoid thundering herd.
    ExponentialWithJitter { factor: f64 },
}

impl BackoffStrategy {
    /// Calculate the delay for a given attempt number (0-indexed).
    pub fn delay(&self, attempt: u32, initial_delay: Duration, max_delay: Duration) -> Duration {
        let delay = match self {
            BackoffStrategy::Constant => initial_delay,
            BackoffStrategy::Linear => initial_delay * (attempt + 1),
            BackoffStrategy::Exponential { factor } => {
                let multiplier = factor.powi(attempt as i32);
                Duration::from_secs_f64(initial_delay.as_secs_f64() * multiplier)
            }
            BackoffStrategy::ExponentialWithJitter { factor } => {
                let base_multiplier = factor.powi(attempt as i32);
                let base_delay = initial_delay.as_secs_f64() * base_multiplier;

                // Add jitter: random value between 0 and base_delay
                let mut rng = rand::thread_rng();
                let jitter = rng.gen::<f64>() * base_delay;

                Duration::from_secs_f64(base_delay + jitter)
            }
        };

        std::cmp::min(delay, max_delay)
    }
}

/// Retry policy that determines when and how to retry.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    config: RetryConfig,
    attempt: u32,
}

impl RetryPolicy {
    /// Create a new retry policy from config.
    pub fn new(config: RetryConfig) -> Self {
        Self { config, attempt: 0 }
    }

    /// Returns the current attempt number (0-indexed).
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Returns true if we should retry after a failure.
    pub fn should_retry(&self) -> bool {
        self.attempt < self.config.max_attempts
    }

    /// Record an attempt and return the delay before the next retry.
    /// Returns None if we've exhausted all retries.
    pub fn next_delay(&mut self, retry_after: Option<Duration>) -> Option<Duration> {
        if !self.should_retry() {
            return None;
        }

        let delay = if let Some(retry_after) = retry_after {
            if self.config.respect_retry_after {
                // Use Retry-After header, but cap it
                std::cmp::min(retry_after, self.config.max_retry_after)
            } else {
                self.config.backoff.delay(
                    self.attempt,
                    self.config.initial_delay,
                    self.config.max_delay,
                )
            }
        } else {
            self.config.backoff.delay(
                self.attempt,
                self.config.initial_delay,
                self.config.max_delay,
            )
        };

        self.attempt += 1;
        Some(delay)
    }

    /// Reset the retry policy for a new request.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert!(config.respect_retry_after);
    }

    #[test]
    fn test_no_retry() {
        let config = RetryConfig::no_retry();
        let policy = RetryPolicy::new(config);
        assert!(!policy.should_retry());
    }

    #[test]
    fn test_constant_backoff() {
        let delay = BackoffStrategy::Constant.delay(
            0,
            Duration::from_secs(1),
            Duration::from_secs(60),
        );
        assert_eq!(delay, Duration::from_secs(1));

        let delay = BackoffStrategy::Constant.delay(
            5,
            Duration::from_secs(1),
            Duration::from_secs(60),
        );
        assert_eq!(delay, Duration::from_secs(1));
    }

    #[test]
    fn test_exponential_backoff() {
        let strategy = BackoffStrategy::Exponential { factor: 2.0 };
        let initial = Duration::from_secs(1);
        let max = Duration::from_secs(60);

        assert_eq!(strategy.delay(0, initial, max), Duration::from_secs(1));
        assert_eq!(strategy.delay(1, initial, max), Duration::from_secs(2));
        assert_eq!(strategy.delay(2, initial, max), Duration::from_secs(4));
        assert_eq!(strategy.delay(3, initial, max), Duration::from_secs(8));

        // Should cap at max
        assert_eq!(strategy.delay(10, initial, max), Duration::from_secs(60));
    }

    #[test]
    fn test_exponential_with_jitter() {
        let strategy = BackoffStrategy::ExponentialWithJitter { factor: 2.0 };
        let initial = Duration::from_secs(1);
        let max = Duration::from_secs(60);

        // With jitter, delay should be between base and 2*base
        let delay = strategy.delay(0, initial, max);
        assert!(delay >= Duration::from_secs(1));
        assert!(delay <= Duration::from_secs(2));

        let delay = strategy.delay(1, initial, max);
        assert!(delay >= Duration::from_secs(2));
        assert!(delay <= Duration::from_secs(4));
    }

    #[test]
    fn test_retry_policy() {
        let config = RetryConfig::default().with_max_attempts(3);
        let mut policy = RetryPolicy::new(config);

        assert!(policy.should_retry());
        assert_eq!(policy.attempt(), 0);

        let delay1 = policy.next_delay(None).unwrap();
        assert_eq!(policy.attempt(), 1);
        assert!(policy.should_retry());

        let delay2 = policy.next_delay(None).unwrap();
        assert_eq!(policy.attempt(), 2);
        assert!(policy.should_retry());

        let delay3 = policy.next_delay(None).unwrap();
        assert_eq!(policy.attempt(), 3);
        assert!(!policy.should_retry());

        // Exponential backoff means delay2 > delay1, delay3 > delay2 (generally)
        // But with jitter, we can't guarantee strict ordering
        assert!(delay1 > Duration::ZERO);
        assert!(delay2 > Duration::ZERO);
        assert!(delay3 > Duration::ZERO);

        // Should return None when exhausted
        assert!(policy.next_delay(None).is_none());
    }

    #[test]
    fn test_retry_after_header() {
        let mut config = RetryConfig::default();
        config.max_retry_after = Duration::from_secs(60);
        let mut policy = RetryPolicy::new(config);

        // Should respect Retry-After
        let delay = policy.next_delay(Some(Duration::from_secs(30))).unwrap();
        assert_eq!(delay, Duration::from_secs(30));

        // Should cap excessive Retry-After
        let delay = policy.next_delay(Some(Duration::from_secs(120))).unwrap();
        assert_eq!(delay, Duration::from_secs(60));
    }

    #[test]
    fn test_policy_reset() {
        let config = RetryConfig::default().with_max_attempts(2);
        let mut policy = RetryPolicy::new(config);

        policy.next_delay(None);
        policy.next_delay(None);
        assert!(!policy.should_retry());

        policy.reset();
        assert!(policy.should_retry());
        assert_eq!(policy.attempt(), 0);
    }
}
