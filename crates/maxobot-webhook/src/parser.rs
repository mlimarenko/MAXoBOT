//! Webhook payload parser to typed update envelopes.

use maxobot_core::{
    errors::api_error::ApiError,
    updates::{
        parser::parse_update,
        update_envelope::{UpdateEnvelope, UpdateSource},
    },
};
use serde_json::Value;
use thiserror::Error;

/// Webhook parse failures.
#[derive(Debug, Error)]
pub enum WebhookParseError {
    /// JSON decode failure.
    #[error("invalid webhook JSON payload: {0}")]
    Json(#[from] serde_json::Error),
    /// Typed update parse failure.
    #[error(transparent)]
    Update(#[from] ApiError),
}

/// Parses webhook bytes into typed envelope.
pub fn parse_webhook_payload(payload: &[u8]) -> Result<UpdateEnvelope, WebhookParseError> {
    let value: Value = serde_json::from_slice(payload)?;
    parse_update(value, UpdateSource::Webhook).map_err(Into::into)
}
