//! Error taxonomy for MAX transport and contract handling.

use std::path::PathBuf;

use http::StatusCode;
use thiserror::Error;

/// Retry categories used by resilience policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// No retry is expected.
    None,
    /// Retry with standard backoff.
    Backoff,
    /// Retry under rate-limit strategy.
    RateLimited,
}

/// Unified SDK error type for API, transport, and validation failures.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Invalid runtime configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Invalid header data.
    #[error("invalid header `{header}`: {reason}")]
    InvalidHeader {
        /// Header name.
        header: String,
        /// Validation reason.
        reason: String,
    },

    /// Query-based auth is forbidden by current contract.
    #[error("query-based authentication is forbidden; use Authorization header")]
    QueryAuthenticationForbidden,

    /// URL/path join failed for the given route.
    #[error("failed to resolve URL for path `{path}` against `{base}`")]
    UrlJoinError {
        /// Base URL used for request.
        base: String,
        /// Path that failed to join.
        path: String,
    },

    /// HTTP transport-level failure.
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// Non-success API response.
    #[error("API request failed with status {status} ({code:?}): {message:?}")]
    HttpStatus {
        /// HTTP status.
        status: StatusCode,
        /// Optional API-level error code.
        code: Option<String>,
        /// Optional message.
        message: Option<String>,
    },

    /// Response JSON parsing failed.
    #[error("failed to decode response body: {source}; body preview: {body_preview}")]
    ResponseDecode {
        /// JSON decode source.
        source: serde_json::Error,
        /// Redacted preview of body.
        body_preview: String,
    },

    /// Response body shape is invalid for expected contract.
    #[error("invalid response shape: {0}")]
    InvalidResponseShape(String),

    /// Parsing update payload failed.
    #[error("invalid update payload: {0}")]
    InvalidUpdatePayload(String),

    /// Filesystem read error for fixture.
    #[error("failed to read fixture `{path}`: {source}")]
    FixtureIo {
        /// Fixture path.
        path: PathBuf,
        /// IO error source.
        source: std::io::Error,
    },

    /// Fixture JSON parse error.
    #[error("failed to parse fixture `{path}`: {source}")]
    FixtureParse {
        /// Fixture path.
        path: PathBuf,
        /// Parse error source.
        source: serde_json::Error,
    },

    /// Fixture does not match expected shape.
    #[error("fixture schema error for `{path}`: {reason}")]
    FixtureSchema {
        /// Fixture path.
        path: PathBuf,
        /// Schema mismatch reason.
        reason: String,
    },
}

impl ApiError {
    /// Constructs a status error.
    #[must_use]
    pub fn from_status(status: StatusCode, code: Option<String>, message: Option<String>) -> Self {
        Self::HttpStatus {
            status,
            code,
            message,
        }
    }

    /// Returns retry class for this error.
    #[must_use]
    pub fn retry_class(&self) -> RetryClass {
        match self {
            Self::HttpStatus {
                status: StatusCode::TOO_MANY_REQUESTS,
                ..
            } => RetryClass::RateLimited,
            Self::HttpStatus { status, .. }
                if matches!(
                    *status,
                    StatusCode::BAD_GATEWAY
                        | StatusCode::SERVICE_UNAVAILABLE
                        | StatusCode::GATEWAY_TIMEOUT
                        | StatusCode::INTERNAL_SERVER_ERROR
                ) =>
            {
                RetryClass::Backoff
            }
            Self::Transport(error) if error.is_connect() || error.is_timeout() => {
                RetryClass::Backoff
            }
            _ => RetryClass::None,
        }
    }

    /// Returns a safe user-facing message with reduced leakage risk.
    #[must_use]
    pub fn redacted_message(&self) -> String {
        redact_sensitive(&self.to_string())
    }
}

/// Redacts common secret markers from textual payloads.
#[must_use]
pub fn redact_sensitive(input: &str) -> String {
    let mut output = input.to_owned();
    for marker in [
        "access_token=",
        "Authorization:",
        "authorization:",
        "token=",
        "secret=",
    ] {
        if let Some(index) = output.find(marker) {
            let start = index + marker.len();
            let end = output[start..]
                .find(['&', ' ', '\n', '\r'])
                .map_or(output.len(), |offset| start + offset);
            output.replace_range(start..end, "***");
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{ApiError, RetryClass, redact_sensitive};

    #[test]
    fn classifies_429_as_rate_limited() {
        let error = ApiError::from_status(http::StatusCode::TOO_MANY_REQUESTS, None, None);
        assert_eq!(error.retry_class(), RetryClass::RateLimited);
    }

    #[test]
    fn redacts_token_markers() {
        let text = "failed request access_token=abc123";
        let redacted = redact_sensitive(text);
        assert!(redacted.contains("access_token=***"));
    }
}
