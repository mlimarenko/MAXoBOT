//! Inbound update mapping primitives.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use maxobot_core::updates::update_envelope::UpdateEnvelope;

use crate::{
    config::adapter_config::{AdapterConfig, UnknownEventPolicy},
    context::adapter_context::AdapterContext,
    errors::translator::{AdapterFailure, BotronFailureClass},
};

/// Lifecycle update mapper.
pub mod lifecycle_mapper;
/// Message/callback mapper.
pub mod message_mapper;
/// Unknown update mapper.
pub mod unknown_mapper;

/// Mapped Botron interaction event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InboundEvent {
    /// Botron interaction event name.
    pub event_name: String,
    /// Adapter context and external IDs.
    pub context: AdapterContext,
    /// Event payload normalized for Botron layer.
    pub payload: Value,
}

/// Maps one MAX update into Botron inbound event according to adapter config.
pub fn map_inbound_update(
    update: &UpdateEnvelope,
    context: AdapterContext,
    config: &AdapterConfig,
) -> Result<Option<InboundEvent>, AdapterFailure> {
    if let Some(event) = message_mapper::map_message_or_callback(update, context.clone())? {
        return Ok(Some(event));
    }
    if let Some(event) = lifecycle_mapper::map_lifecycle_update(update, context.clone())? {
        return Ok(Some(event));
    }

    match config.unknown_event_policy {
        UnknownEventPolicy::EmitAsUnknown => Ok(Some(unknown_mapper::map_unknown_update(
            update, context, config,
        ))),
        UnknownEventPolicy::DropWithWarning => {
            tracing::warn!(
                update_type = update.update_type.as_str(),
                "dropping unknown MAX update due to adapter policy",
            );
            Ok(None)
        }
        UnknownEventPolicy::FailFast => Err(AdapterFailure::new(
            BotronFailureClass::ChannelUnknownEvent,
            format!("unknown update type `{}`", update.update_type.as_str()),
        )),
    }
}

fn payload_object(update: &UpdateEnvelope) -> Option<&serde_json::Map<String, Value>> {
    update
        .payload
        .get("payload")
        .and_then(Value::as_object)
        .or_else(|| update.payload.as_object())
}

fn payload_string(payload: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    payload.get(key).and_then(value_to_string)
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}
