use std::{collections::BTreeSet, time::Duration};

use thiserror::Error;

/// Default total attempt count (initial call + retries).
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

/// Error returned for invalid backoff strategy configuration.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BackoffStrategyError {
    /// Fixed delay must be positive.
    #[error("fixed delay must be greater than zero")]
    ZeroFixedDelay,
    /// Exponential initial delay must be positive.
    #[error("exponential initial delay must be greater than zero")]
    ZeroInitialDelay,
    /// Exponential multiplier must be positive.
    #[error("exponential multiplier must be greater than zero")]
    ZeroMultiplier,
    /// Exponential maximum delay cannot be lower than initial delay.
    #[error("exponential max delay ({max_delay:?}) must be >= initial delay ({initial_delay:?})")]
    MaxDelayLessThanInitial {
        /// Initial delay.
        initial_delay: Duration,
        /// Maximum delay.
        max_delay: Duration,
    },
}

/// Error returned for invalid jitter strategy configuration.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum JitterError {
    /// Full jitter requires a non-zero maximum.
    #[error("full jitter max value must be greater than zero")]
    ZeroMaxJitter,
}

/// Errors returned by retry policy validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RetryPolicyError {
    /// Maximum attempts must be positive.
    #[error("max attempts must be greater than zero")]
    ZeroMaxAttempts,
    /// At least one retry class must be enabled.
    #[error("retry class filter must include at least one class")]
    EmptyRetryClassFilter,
    /// Backoff strategy validation failed.
    #[error("invalid backoff strategy: {0}")]
    InvalidBackoff(#[from] BackoffStrategyError),
    /// Jitter strategy validation failed.
    #[error("invalid jitter strategy: {0}")]
    InvalidJitter(#[from] JitterError),
}

/// Retry class categories used for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RetryClass {
    /// Non-retryable failures.
    None,
    /// Retryable failures that can be repeated immediately.
    Immediate,
    /// Retryable failures that should use backoff.
    Backoff,
    /// Rate-limited failures (`429`) that should retry with delay.
    RateLimited,
    /// Transport-level transient failures (timeouts, resets).
    Transport,
    /// MAX delayed-media processing failure (`attachment.not.ready`).
    AttachmentNotReady,
}

/// Filter that controls which retry classes are retried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryClassFilter {
    allowed_classes: BTreeSet<RetryClass>,
}

impl RetryClassFilter {
    /// Creates a class filter from a class iterator.
    #[must_use]
    pub fn new(classes: impl IntoIterator<Item = RetryClass>) -> Self {
        let allowed_classes = classes.into_iter().collect();
        Self { allowed_classes }
    }

    /// Returns the default class filter for transient failures.
    #[must_use]
    pub fn transient_defaults() -> Self {
        Self::new([
            RetryClass::Immediate,
            RetryClass::Backoff,
            RetryClass::RateLimited,
            RetryClass::Transport,
            RetryClass::AttachmentNotReady,
        ])
    }

    /// Checks whether a retry class is enabled.
    #[must_use]
    pub fn allows(&self, class: RetryClass) -> bool {
        self.allowed_classes.contains(&class)
    }

    /// Returns `true` when no classes are enabled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.allowed_classes.is_empty()
    }
}

impl Default for RetryClassFilter {
    fn default() -> Self {
        Self::transient_defaults()
    }
}

/// Strategy for computing delay between retries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackoffStrategy {
    /// Fixed delay for every retry attempt.
    Fixed {
        /// Delay applied on each retry.
        delay: Duration,
    },
    /// Exponential delay that grows by multiplier and is capped.
    Exponential {
        /// Delay for the first retry.
        initial_delay: Duration,
        /// Growth factor for every subsequent retry.
        multiplier: u32,
        /// Maximum delay cap.
        max_delay: Duration,
    },
}

impl BackoffStrategy {
    /// Validates strategy parameters.
    pub fn validate(&self) -> Result<(), BackoffStrategyError> {
        match self {
            Self::Fixed { delay } => {
                if delay.is_zero() {
                    return Err(BackoffStrategyError::ZeroFixedDelay);
                }
            }
            Self::Exponential {
                initial_delay,
                multiplier,
                max_delay,
            } => {
                if initial_delay.is_zero() {
                    return Err(BackoffStrategyError::ZeroInitialDelay);
                }
                if *multiplier == 0 {
                    return Err(BackoffStrategyError::ZeroMultiplier);
                }
                if max_delay < initial_delay {
                    return Err(BackoffStrategyError::MaxDelayLessThanInitial {
                        initial_delay: *initial_delay,
                        max_delay: *max_delay,
                    });
                }
            }
        }

        Ok(())
    }

    /// Computes delay for the provided retry number (starting from 1).
    #[must_use]
    pub fn delay_for_retry(&self, retry_number: u32) -> Duration {
        let retry_number = retry_number.max(1);
        match self {
            Self::Fixed { delay } => *delay,
            Self::Exponential {
                initial_delay,
                multiplier,
                max_delay,
            } => {
                let exponent = retry_number.saturating_sub(1);
                let growth = multiplier.saturating_pow(exponent);
                initial_delay.saturating_mul(growth).min(*max_delay)
            }
        }
    }
}

/// Jitter configuration applied to computed backoff delay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Jitter {
    /// Disable jitter.
    None,
    /// Full jitter in range `0..=max_jitter`.
    Full {
        /// Maximum random jitter value.
        max_jitter: Duration,
    },
}

impl Jitter {
    /// Validates jitter configuration.
    pub fn validate(&self) -> Result<(), JitterError> {
        match self {
            Self::None => Ok(()),
            Self::Full { max_jitter } => {
                if max_jitter.is_zero() {
                    return Err(JitterError::ZeroMaxJitter);
                }
                Ok(())
            }
        }
    }
}

/// Retry policy controls attempts, class filters, backoff, and jitter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of attempts including the first call.
    pub max_attempts: u32,
    /// Backoff strategy used between retries.
    pub backoff: BackoffStrategy,
    /// Jitter strategy used with backoff.
    pub jitter: Jitter,
    /// Class filter that gates which failures are retryable.
    pub retry_classes: RetryClassFilter,
}

impl RetryPolicy {
    /// Validates policy settings.
    pub fn validate(&self) -> Result<(), RetryPolicyError> {
        if self.max_attempts == 0 {
            return Err(RetryPolicyError::ZeroMaxAttempts);
        }
        if self.retry_classes.is_empty() {
            return Err(RetryPolicyError::EmptyRetryClassFilter);
        }
        self.backoff.validate()?;
        self.jitter.validate()?;
        Ok(())
    }

    /// Returns whether another retry is allowed.
    ///
    /// `attempts_made` is the number of attempts already performed.
    #[must_use]
    pub fn should_retry(&self, class: RetryClass, attempts_made: u32) -> bool {
        attempts_made < self.max_attempts && self.retry_classes.allows(class)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            backoff: BackoffStrategy::Exponential {
                initial_delay: Duration::from_millis(250),
                multiplier: 2,
                max_delay: Duration::from_secs(8),
            },
            jitter: Jitter::Full {
                max_jitter: Duration::from_millis(250),
            },
            retry_classes: RetryClassFilter::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        BackoffStrategy, Jitter, JitterError, RetryClass, RetryClassFilter, RetryPolicy,
        RetryPolicyError,
    };

    #[test]
    fn default_policy_is_valid_and_transient_focused() {
        let policy = RetryPolicy::default();

        assert_eq!(policy.max_attempts, 5);
        assert!(policy.retry_classes.allows(RetryClass::RateLimited));
        assert!(policy.retry_classes.allows(RetryClass::Transport));
        assert!(policy.retry_classes.allows(RetryClass::AttachmentNotReady));
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn should_retry_checks_attempt_budget_and_class_filter() {
        let policy = RetryPolicy::default();

        assert!(policy.should_retry(RetryClass::Backoff, 1));
        assert!(!policy.should_retry(RetryClass::None, 1));
        assert!(!policy.should_retry(RetryClass::Backoff, policy.max_attempts));
    }

    #[test]
    fn exponential_delay_grows_and_is_capped() {
        let strategy = BackoffStrategy::Exponential {
            initial_delay: Duration::from_millis(100),
            multiplier: 2,
            max_delay: Duration::from_millis(350),
        };

        assert_eq!(strategy.delay_for_retry(1), Duration::from_millis(100));
        assert_eq!(strategy.delay_for_retry(2), Duration::from_millis(200));
        assert_eq!(strategy.delay_for_retry(3), Duration::from_millis(350));
        assert_eq!(strategy.delay_for_retry(10), Duration::from_millis(350));
    }

    #[test]
    fn validation_rejects_empty_retry_class_filter() {
        let policy = RetryPolicy {
            retry_classes: RetryClassFilter::new([]),
            ..RetryPolicy::default()
        };

        assert_eq!(
            policy.validate(),
            Err(RetryPolicyError::EmptyRetryClassFilter)
        );
    }

    #[test]
    fn validation_rejects_zero_max_attempts() {
        let policy = RetryPolicy {
            max_attempts: 0,
            ..RetryPolicy::default()
        };

        assert_eq!(policy.validate(), Err(RetryPolicyError::ZeroMaxAttempts));
    }

    #[test]
    fn validation_rejects_invalid_backoff_strategy() {
        let policy = RetryPolicy {
            backoff: BackoffStrategy::Fixed {
                delay: Duration::ZERO,
            },
            ..RetryPolicy::default()
        };

        assert!(matches!(
            policy.validate(),
            Err(RetryPolicyError::InvalidBackoff(_))
        ));
    }

    #[test]
    fn jitter_validation_rejects_zero_full_jitter() {
        let jitter = Jitter::Full {
            max_jitter: Duration::ZERO,
        };

        assert_eq!(jitter.validate(), Err(JitterError::ZeroMaxJitter));
    }
}
