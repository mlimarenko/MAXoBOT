//! Deterministic translation from SDK failures to Botron classes.

use thiserror::Error;

use maxobot_core::errors::api_error::{ApiError, RetryClass};

/// Botron-facing failure classes for channel integrations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotronFailureClass {
    /// Authentication or authorization failure.
    ChannelAuthFailed,
    /// Provider-side rate limiting.
    ChannelRateLimited,
    /// Temporary provider unavailability.
    ChannelProviderUnavailable,
    /// Contract or payload validation issue.
    ChannelContractError,
    /// Unknown inbound event type.
    ChannelUnknownEvent,
    /// Transport/network failure.
    ChannelTransportError,
    /// Internal adapter/runtime issue.
    ChannelInternalError,
}

/// Adapter error with deterministic failure class.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{class:?}: {message}")]
pub struct AdapterFailure {
    /// Botron failure class.
    pub class: BotronFailureClass,
    /// Failure message.
    pub message: String,
}

impl AdapterFailure {
    /// Creates typed failure.
    #[must_use]
    pub fn new(class: BotronFailureClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }
}

/// Maps SDK API error to Botron failure class.
#[must_use]
pub fn translate_api_error(error: &ApiError) -> BotronFailureClass {
    match error {
        ApiError::QueryAuthenticationForbidden | ApiError::InvalidHeader { .. } => {
            BotronFailureClass::ChannelAuthFailed
        }
        ApiError::HttpStatus { status, .. } if *status == http::StatusCode::UNAUTHORIZED => {
            BotronFailureClass::ChannelAuthFailed
        }
        ApiError::HttpStatus {
            status: http::StatusCode::TOO_MANY_REQUESTS,
            ..
        } => BotronFailureClass::ChannelRateLimited,
        ApiError::Transport(_) => BotronFailureClass::ChannelTransportError,
        ApiError::InvalidConfiguration(_)
        | ApiError::InvalidResponseShape(_)
        | ApiError::InvalidUpdatePayload(_)
        | ApiError::ResponseDecode { .. } => BotronFailureClass::ChannelContractError,
        ApiError::HttpStatus { .. } => match error.retry_class() {
            RetryClass::RateLimited => BotronFailureClass::ChannelRateLimited,
            RetryClass::Backoff => BotronFailureClass::ChannelProviderUnavailable,
            RetryClass::None => BotronFailureClass::ChannelContractError,
        },
        ApiError::FixtureIo { .. }
        | ApiError::FixtureParse { .. }
        | ApiError::FixtureSchema { .. }
        | ApiError::UrlJoinError { .. } => BotronFailureClass::ChannelInternalError,
    }
}

/// Converts SDK error into typed adapter failure.
#[must_use]
pub fn to_adapter_failure(error: &ApiError) -> AdapterFailure {
    AdapterFailure::new(translate_api_error(error), error.redacted_message())
}
