//! Webhook request secret verification.

use http::HeaderMap;
use thiserror::Error;

/// Default MAX webhook secret header.
pub const DEFAULT_SECRET_HEADER: &str = "x-max-bot-api-secret";

/// Webhook verifier failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WebhookVerifyError {
    /// Secret header is required but missing.
    #[error("webhook secret header `{header}` is missing")]
    MissingSecretHeader {
        /// Expected header name.
        header: String,
    },
    /// Secret value does not match.
    #[error("webhook secret is invalid")]
    InvalidSecret,
    /// Header is present but value is not valid UTF-8.
    #[error("webhook secret header value is not valid UTF-8")]
    InvalidHeaderValue,
}

/// Webhook secret verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookVerifier {
    expected_secret: Option<String>,
    header_name: String,
}

impl WebhookVerifier {
    /// Creates verifier using default header name.
    #[must_use]
    pub fn new(expected_secret: Option<String>) -> Self {
        Self {
            expected_secret,
            header_name: DEFAULT_SECRET_HEADER.to_owned(),
        }
    }

    /// Creates verifier with explicit header name.
    #[must_use]
    pub fn with_header_name(
        expected_secret: Option<String>,
        header_name: impl Into<String>,
    ) -> Self {
        Self {
            expected_secret,
            header_name: header_name.into(),
        }
    }

    /// Verifies webhook headers against configured secret.
    pub fn verify(&self, headers: &HeaderMap) -> Result<(), WebhookVerifyError> {
        let Some(expected_secret) = self.expected_secret.as_deref() else {
            return Ok(());
        };

        let value = headers.get(&self.header_name).ok_or_else(|| {
            WebhookVerifyError::MissingSecretHeader {
                header: self.header_name.clone(),
            }
        })?;

        let actual = value
            .to_str()
            .map_err(|_| WebhookVerifyError::InvalidHeaderValue)?;
        if actual != expected_secret {
            return Err(WebhookVerifyError::InvalidSecret);
        }

        Ok(())
    }

    /// Returns expected header name.
    #[must_use]
    pub fn header_name(&self) -> &str {
        &self.header_name
    }
}

#[cfg(test)]
mod tests {
    use http::{HeaderMap, HeaderValue};

    use super::{DEFAULT_SECRET_HEADER, WebhookVerifier, WebhookVerifyError};

    #[test]
    fn verifies_valid_secret() {
        let verifier = WebhookVerifier::new(Some("secret123".to_owned()));
        let mut headers = HeaderMap::new();
        headers.insert(DEFAULT_SECRET_HEADER, HeaderValue::from_static("secret123"));

        assert_eq!(verifier.verify(&headers), Ok(()));
    }

    #[test]
    fn rejects_missing_secret_header() {
        let verifier = WebhookVerifier::new(Some("secret123".to_owned()));
        let headers = HeaderMap::new();
        assert!(matches!(
            verifier.verify(&headers),
            Err(WebhookVerifyError::MissingSecretHeader { .. })
        ));
    }
}
