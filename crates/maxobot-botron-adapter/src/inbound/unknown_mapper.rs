//! Mapper for unknown/future update types.

use serde_json::json;

use maxobot_core::updates::update_envelope::UpdateEnvelope;

use crate::{
    config::adapter_config::AdapterConfig, context::adapter_context::AdapterContext,
    inbound::InboundEvent,
};

/// Maps unknown update into `interaction.channel.unknown`.
#[must_use]
pub fn map_unknown_update(
    update: &UpdateEnvelope,
    mut context: AdapterContext,
    config: &AdapterConfig,
) -> InboundEvent {
    context.external_ids.update_type = Some(update.update_type.as_str().to_owned());
    let raw_payload = if config.include_raw_payload {
        update.raw.clone()
    } else {
        json!(null)
    };

    InboundEvent {
        event_name: "interaction.channel.unknown".to_owned(),
        context,
        payload: json!({
            "raw_update_type": update.update_type.as_str(),
            "raw_payload": raw_payload,
            "timestamp": update.timestamp,
        }),
    }
}
