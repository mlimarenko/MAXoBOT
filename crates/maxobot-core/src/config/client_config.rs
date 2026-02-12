use std::time::Duration;

use thiserror::Error;
use url::Url;

use crate::{
    client::endpoint_resolver::{
        DEFAULT_API_BASE_URL, EndpointResolver, EndpointResolverError, validate_api_base_url,
    },
    reliability::{
        rate_limit_policy::{RateLimitPolicy, RateLimitPolicyError},
        retry_policy::{RetryPolicy, RetryPolicyError},
    },
};

/// Default timeout for a single API request.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Runtime mode flags that control update ingestion paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientModeFlags {
    /// Enable webhook-based updates.
    pub webhook_enabled: bool,
    /// Enable long-polling updates.
    pub polling_enabled: bool,
}

impl ClientModeFlags {
    /// Creates a webhook-only mode (production-first default).
    #[must_use]
    pub const fn webhook_only() -> Self {
        Self {
            webhook_enabled: true,
            polling_enabled: false,
        }
    }

    /// Creates a polling-only mode.
    #[must_use]
    pub const fn polling_only() -> Self {
        Self {
            webhook_enabled: false,
            polling_enabled: true,
        }
    }

    /// Creates a mode that allows both webhook and polling.
    #[must_use]
    pub const fn hybrid() -> Self {
        Self {
            webhook_enabled: true,
            polling_enabled: true,
        }
    }
}

impl Default for ClientModeFlags {
    fn default() -> Self {
        Self::webhook_only()
    }
}

/// Validation errors for [`ClientConfig`].
#[derive(Debug, Error)]
pub enum ClientConfigValidationError {
    /// Invalid API base URL.
    #[error("invalid API base URL: {0}")]
    InvalidApiBaseUrl(#[from] EndpointResolverError),
    /// Timeout must be positive.
    #[error("request timeout must be greater than zero")]
    ZeroRequestTimeout,
    /// User-agent cannot be blank.
    #[error("user agent must not be empty")]
    EmptyUserAgent,
    /// At least one ingestion mode must be enabled.
    #[error("at least one mode must be enabled: webhook or polling")]
    NoEnabledModes,
    /// Retry policy did not pass validation.
    #[error("invalid retry policy: {0}")]
    InvalidRetryPolicy(#[from] RetryPolicyError),
    /// Rate-limit policy did not pass validation.
    #[error("invalid rate-limit policy: {0}")]
    InvalidRateLimitPolicy(#[from] RateLimitPolicyError),
}

/// SDK client runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConfig {
    /// MAX API base URL.
    pub api_base_url: Url,
    /// Request timeout per API call.
    pub request_timeout: Duration,
    /// Retry configuration for retry-classified failures.
    pub retry_policy: RetryPolicy,
    /// Outbound request throttling policy.
    pub rate_limit_policy: RateLimitPolicy,
    /// User-agent used in outbound requests.
    pub user_agent: String,
    /// Webhook/polling runtime mode flags.
    pub mode: ClientModeFlags,
}

impl ClientConfig {
    /// Creates a config using defaults aligned with MAX docs/guidance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a default user-agent value.
    #[must_use]
    pub fn default_user_agent() -> String {
        format!("maxobot-core/{}", env!("CARGO_PKG_VERSION"))
    }

    /// Validates config values for early fail-fast startup checks.
    pub fn validate(&self) -> Result<(), ClientConfigValidationError> {
        validate_api_base_url(&self.api_base_url)?;

        if self.request_timeout.is_zero() {
            return Err(ClientConfigValidationError::ZeroRequestTimeout);
        }

        if self.user_agent.trim().is_empty() {
            return Err(ClientConfigValidationError::EmptyUserAgent);
        }

        if !self.mode.webhook_enabled && !self.mode.polling_enabled {
            return Err(ClientConfigValidationError::NoEnabledModes);
        }

        self.retry_policy.validate()?;
        self.rate_limit_policy.validate()?;

        Ok(())
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        let api_base_url = EndpointResolver::with_override(DEFAULT_API_BASE_URL)
            .expect("DEFAULT_API_BASE_URL must be a valid HTTPS URL")
            .base_url()
            .clone();

        Self {
            api_base_url,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            retry_policy: RetryPolicy::default(),
            rate_limit_policy: RateLimitPolicy::default(),
            user_agent: Self::default_user_agent(),
            mode: ClientModeFlags::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ClientConfig, ClientConfigValidationError, ClientModeFlags};
    use crate::reliability::{
        rate_limit_policy::RateLimitPolicy,
        retry_policy::{RetryClassFilter, RetryPolicy},
    };

    #[test]
    fn default_config_is_valid() {
        let config = ClientConfig::default();

        assert!(config.validate().is_ok());
        assert_eq!(config.api_base_url.as_str(), "https://platform-api.max.ru/");
        assert!(config.mode.webhook_enabled);
        assert!(!config.mode.polling_enabled);
    }

    #[test]
    fn validation_rejects_non_https_base_url() {
        let mut config = ClientConfig::default();
        config.api_base_url = url::Url::parse("http://platform-api.max.ru").expect("valid URL");

        assert!(matches!(
            config.validate(),
            Err(ClientConfigValidationError::InvalidApiBaseUrl(_))
        ));
    }

    #[test]
    fn validation_rejects_zero_timeout() {
        let mut config = ClientConfig::default();
        config.request_timeout = Duration::ZERO;

        assert!(matches!(
            config.validate(),
            Err(ClientConfigValidationError::ZeroRequestTimeout)
        ));
    }

    #[test]
    fn validation_rejects_empty_user_agent() {
        let mut config = ClientConfig::default();
        config.user_agent = "   ".to_owned();

        assert!(matches!(
            config.validate(),
            Err(ClientConfigValidationError::EmptyUserAgent)
        ));
    }

    #[test]
    fn validation_rejects_when_all_modes_disabled() {
        let mut config = ClientConfig::default();
        config.mode = ClientModeFlags {
            webhook_enabled: false,
            polling_enabled: false,
        };

        assert!(matches!(
            config.validate(),
            Err(ClientConfigValidationError::NoEnabledModes)
        ));
    }

    #[test]
    fn validation_rejects_invalid_nested_retry_policy() {
        let mut config = ClientConfig::default();
        config.retry_policy = RetryPolicy {
            retry_classes: RetryClassFilter::new([]),
            ..RetryPolicy::default()
        };

        assert!(matches!(
            config.validate(),
            Err(ClientConfigValidationError::InvalidRetryPolicy(_))
        ));
    }

    #[test]
    fn validation_rejects_invalid_nested_rate_limit_policy() {
        let mut config = ClientConfig::default();
        let mut rate_limit_policy = RateLimitPolicy::default();
        rate_limit_policy.acquire_timeout = Duration::ZERO;
        config.rate_limit_policy = rate_limit_policy;

        assert!(matches!(
            config.validate(),
            Err(ClientConfigValidationError::InvalidRateLimitPolicy(_))
        ));
    }
}
