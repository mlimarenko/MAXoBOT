use std::time::Duration;

use thiserror::Error;

/// MAX guidance default request rate limit in requests per second.
pub const DEFAULT_RATE_LIMIT_RPS: u32 = 30;

/// Default token-bucket capacity aligned with the MAX 30 rps guidance.
pub const DEFAULT_BUCKET_CAPACITY: u32 = DEFAULT_RATE_LIMIT_RPS;

/// Default wait time for acquiring a rate-limit token.
pub const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_millis(250);

/// Errors returned by rate-limit policy validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RateLimitPolicyError {
    /// Token-bucket capacity must be positive.
    #[error("token bucket capacity must be greater than zero")]
    ZeroCapacity,
    /// Token refill rate must be positive.
    #[error("token refill rate must be greater than zero")]
    ZeroRefillPerSecond,
    /// Initial tokens cannot exceed bucket capacity.
    #[error("initial tokens ({initial_tokens}) cannot exceed capacity ({capacity})")]
    InitialTokensExceedCapacity {
        /// Invalid initial token count.
        initial_tokens: u32,
        /// Configured capacity.
        capacity: u32,
    },
    /// Acquire timeout must be positive.
    #[error("token acquire timeout must be greater than zero")]
    ZeroAcquireTimeout,
}

/// Token-bucket settings used for outbound request throttling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenBucketPolicy {
    /// Maximum number of tokens that may be buffered.
    pub capacity: u32,
    /// Number of tokens added each second.
    pub refill_per_second: u32,
    /// Initial token count available at startup.
    pub initial_tokens: u32,
}

impl TokenBucketPolicy {
    /// Creates a token-bucket policy from raw values.
    #[must_use]
    pub const fn new(capacity: u32, refill_per_second: u32, initial_tokens: u32) -> Self {
        Self {
            capacity,
            refill_per_second,
            initial_tokens,
        }
    }

    /// Validates that the token-bucket policy is internally consistent.
    pub fn validate(&self) -> Result<(), RateLimitPolicyError> {
        if self.capacity == 0 {
            return Err(RateLimitPolicyError::ZeroCapacity);
        }

        if self.refill_per_second == 0 {
            return Err(RateLimitPolicyError::ZeroRefillPerSecond);
        }

        if self.initial_tokens > self.capacity {
            return Err(RateLimitPolicyError::InitialTokensExceedCapacity {
                initial_tokens: self.initial_tokens,
                capacity: self.capacity,
            });
        }

        Ok(())
    }
}

impl Default for TokenBucketPolicy {
    fn default() -> Self {
        Self::new(
            DEFAULT_BUCKET_CAPACITY,
            DEFAULT_RATE_LIMIT_RPS,
            DEFAULT_BUCKET_CAPACITY,
        )
    }
}

/// Request-throttling policy for outbound SDK calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitPolicy {
    /// Enables or disables limiter enforcement.
    pub enabled: bool,
    /// Token-bucket settings.
    pub token_bucket: TokenBucketPolicy,
    /// Maximum wait for a token acquisition attempt.
    pub acquire_timeout: Duration,
}

impl RateLimitPolicy {
    /// Creates a rate-limit policy using the given token-bucket settings.
    #[must_use]
    pub fn new(token_bucket: TokenBucketPolicy) -> Self {
        Self {
            enabled: true,
            token_bucket,
            acquire_timeout: DEFAULT_ACQUIRE_TIMEOUT,
        }
    }

    /// Returns configured sustained request rate in requests per second.
    #[must_use]
    pub fn requests_per_second(&self) -> u32 {
        self.token_bucket.refill_per_second
    }

    /// Validates policy consistency.
    pub fn validate(&self) -> Result<(), RateLimitPolicyError> {
        self.token_bucket.validate()?;

        if self.acquire_timeout.is_zero() {
            return Err(RateLimitPolicyError::ZeroAcquireTimeout);
        }

        Ok(())
    }
}

impl Default for RateLimitPolicy {
    fn default() -> Self {
        Self::new(TokenBucketPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{DEFAULT_RATE_LIMIT_RPS, RateLimitPolicy, RateLimitPolicyError, TokenBucketPolicy};

    #[test]
    fn default_policy_is_aligned_with_max_guidance() {
        let policy = RateLimitPolicy::default();

        assert!(policy.enabled);
        assert_eq!(policy.requests_per_second(), DEFAULT_RATE_LIMIT_RPS);
        assert_eq!(policy.token_bucket.capacity, DEFAULT_RATE_LIMIT_RPS);
        assert_eq!(policy.token_bucket.initial_tokens, DEFAULT_RATE_LIMIT_RPS);
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn token_bucket_validation_rejects_zero_capacity() {
        let bucket = TokenBucketPolicy::new(0, 30, 0);

        assert_eq!(bucket.validate(), Err(RateLimitPolicyError::ZeroCapacity));
    }

    #[test]
    fn token_bucket_validation_rejects_zero_refill() {
        let bucket = TokenBucketPolicy::new(30, 0, 0);

        assert_eq!(
            bucket.validate(),
            Err(RateLimitPolicyError::ZeroRefillPerSecond)
        );
    }

    #[test]
    fn token_bucket_validation_rejects_initial_tokens_above_capacity() {
        let bucket = TokenBucketPolicy::new(30, 30, 31);

        assert_eq!(
            bucket.validate(),
            Err(RateLimitPolicyError::InitialTokensExceedCapacity {
                initial_tokens: 31,
                capacity: 30,
            })
        );
    }

    #[test]
    fn policy_validation_rejects_zero_acquire_timeout() {
        let mut policy = RateLimitPolicy::default();
        policy.acquire_timeout = Duration::ZERO;

        assert_eq!(
            policy.validate(),
            Err(RateLimitPolicyError::ZeroAcquireTimeout)
        );
    }
}
