//! Credential primitives for authenticated API calls and webhook verification.

use core::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Minimum allowed webhook secret length.
pub const WEBHOOK_SECRET_MIN_LEN: usize = 5;

/// Maximum allowed webhook secret length.
pub const WEBHOOK_SECRET_MAX_LEN: usize = 256;

const REDACTED_VALUE: &str = "***REDACTED***";

/// Bot credentials used by the SDK runtime.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotCredentials {
    token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    webhook_secret: Option<String>,
}

/// Validation failures for [`BotCredentials`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CredentialsValidationError {
    /// Bot token is empty or whitespace-only.
    #[error("bot token must not be empty")]
    EmptyToken,

    /// Webhook secret is outside supported length bounds.
    #[error(
        "webhook secret length must be between {WEBHOOK_SECRET_MIN_LEN} and {WEBHOOK_SECRET_MAX_LEN} characters, got {length}"
    )]
    InvalidWebhookSecretLength {
        /// Observed secret length.
        length: usize,
    },

    /// Webhook secret contains a character outside `[a-zA-Z0-9_-]`.
    #[error("webhook secret contains invalid character '{character}' at index {index}")]
    InvalidWebhookSecretCharacter {
        /// Zero-based index of the invalid character.
        index: usize,
        /// Invalid character value.
        character: char,
    },
}

impl BotCredentials {
    /// Creates credentials from token and optional webhook secret.
    pub fn from_parts(
        token: impl Into<String>,
        webhook_secret: Option<String>,
    ) -> Result<Self, CredentialsValidationError> {
        let token = token.into();
        Self::validate_token(&token)?;
        Self::validate_webhook_secret(webhook_secret.as_deref())?;

        Ok(Self {
            token,
            webhook_secret,
        })
    }

    /// Creates credentials with token only.
    pub fn new(token: impl Into<String>) -> Result<Self, CredentialsValidationError> {
        Self::from_parts(token, None)
    }

    /// Returns the raw bot token.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Returns webhook secret, if configured.
    pub fn webhook_secret(&self) -> Option<&str> {
        self.webhook_secret.as_deref()
    }

    /// Returns `true` when a webhook secret is configured.
    pub fn has_webhook_secret(&self) -> bool {
        self.webhook_secret.is_some()
    }

    /// Sets or clears webhook secret and re-validates credentials.
    pub fn set_webhook_secret(
        &mut self,
        webhook_secret: Option<String>,
    ) -> Result<(), CredentialsValidationError> {
        Self::validate_webhook_secret(webhook_secret.as_deref())?;
        self.webhook_secret = webhook_secret;
        Ok(())
    }

    /// Returns a new instance with webhook secret set.
    pub fn with_webhook_secret(
        mut self,
        webhook_secret: impl Into<String>,
    ) -> Result<Self, CredentialsValidationError> {
        self.set_webhook_secret(Some(webhook_secret.into()))?;
        Ok(self)
    }

    /// Validates credential fields.
    pub fn validate(&self) -> Result<(), CredentialsValidationError> {
        Self::validate_token(&self.token)?;
        Self::validate_webhook_secret(self.webhook_secret.as_deref())
    }

    /// Validates token string.
    pub fn validate_token(token: &str) -> Result<(), CredentialsValidationError> {
        if token.trim().is_empty() {
            return Err(CredentialsValidationError::EmptyToken);
        }

        Ok(())
    }

    /// Validates webhook secret according to platform constraints.
    pub fn validate_webhook_secret(
        webhook_secret: Option<&str>,
    ) -> Result<(), CredentialsValidationError> {
        if let Some(secret) = webhook_secret {
            let length = secret.chars().count();
            if !(WEBHOOK_SECRET_MIN_LEN..=WEBHOOK_SECRET_MAX_LEN).contains(&length) {
                return Err(CredentialsValidationError::InvalidWebhookSecretLength { length });
            }

            if let Some((index, character)) = secret.chars().enumerate().find(|(_, character)| {
                !character.is_ascii_alphanumeric() && *character != '_' && *character != '-'
            }) {
                return Err(CredentialsValidationError::InvalidWebhookSecretCharacter {
                    index,
                    character,
                });
            }
        }

        Ok(())
    }
}

impl fmt::Debug for BotCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = formatter.debug_struct("BotCredentials");
        builder.field("token", &REDACTED_VALUE);

        if self.has_webhook_secret() {
            builder.field("webhook_secret", &REDACTED_VALUE);
        } else {
            builder.field("webhook_secret", &None::<&str>);
        }

        builder.finish()
    }
}

impl fmt::Display for BotCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let webhook_secret = if self.has_webhook_secret() {
            REDACTED_VALUE
        } else {
            "<none>"
        };

        write!(
            formatter,
            "BotCredentials(token={REDACTED_VALUE}, webhook_secret={webhook_secret})"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{BotCredentials, CredentialsValidationError};

    #[test]
    fn token_must_not_be_empty() {
        let error = BotCredentials::new("   ").expect_err("token should be validated");
        assert_eq!(error, CredentialsValidationError::EmptyToken);
    }

    #[test]
    fn webhook_secret_must_match_pattern() {
        let error = BotCredentials::from_parts("token", Some("bad!secret".to_owned()))
            .expect_err("secret should be validated");

        assert!(matches!(
            error,
            CredentialsValidationError::InvalidWebhookSecretCharacter { .. }
        ));
    }

    #[test]
    fn debug_and_display_redact_sensitive_data() {
        let credentials =
            BotCredentials::from_parts("secret-token", Some("safe_secret".to_owned()))
                .expect("credentials should be valid");

        let debug = format!("{credentials:?}");
        let display = credentials.to_string();

        assert!(!debug.contains("secret-token"));
        assert!(!debug.contains("safe_secret"));
        assert!(!display.contains("secret-token"));
        assert!(!display.contains("safe_secret"));
        assert!(debug.contains("***REDACTED***"));
        assert!(display.contains("***REDACTED***"));
    }

    #[test]
    fn serde_round_trip_keeps_fields() {
        let credentials = BotCredentials::from_parts("token", Some("valid_secret".to_owned()))
            .expect("credentials should be valid");

        let encoded = serde_json::to_string(&credentials).expect("should serialize");
        let decoded: BotCredentials = serde_json::from_str(&encoded).expect("should deserialize");

        assert_eq!(decoded.token(), "token");
        assert_eq!(decoded.webhook_secret(), Some("valid_secret"));
    }
}
