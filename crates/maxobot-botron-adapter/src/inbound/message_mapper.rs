//! Inbound mapper for message and callback updates.

use serde_json::json;

use maxobot_core::updates::update_envelope::{KnownUpdateType, UpdateEnvelope, UpdateType};

use crate::{
    context::adapter_context::AdapterContext,
    errors::translator::{AdapterFailure, BotronFailureClass},
    inbound::{InboundEvent, payload_object, payload_string},
};

/// Maps `message_created` and `message_callback` updates.
pub fn map_message_or_callback(
    update: &UpdateEnvelope,
    mut context: AdapterContext,
) -> Result<Option<InboundEvent>, AdapterFailure> {
    let Some(payload) = payload_object(update) else {
        return Err(AdapterFailure::new(
            BotronFailureClass::ChannelContractError,
            "update payload is not an object",
        ));
    };

    match &update.update_type {
        UpdateType::Known(KnownUpdateType::MessageCreated) => {
            context.external_ids.update_type = Some("message_created".to_owned());
            context.external_ids.chat_id = payload_string(payload, "chat_id");
            context.external_ids.user_id = payload_string(payload, "user_id");
            context.external_ids.message_id = payload_string(payload, "message_id");

            Ok(Some(InboundEvent {
                event_name: "interaction.message.received".to_owned(),
                context,
                payload: json!({
                    "channel_chat_id": payload.get("chat_id"),
                    "channel_user_id": payload.get("user_id"),
                    "message_id": payload.get("message_id"),
                    "timestamp": update.timestamp,
                    "raw": update.payload,
                }),
            }))
        }
        UpdateType::Known(KnownUpdateType::MessageCallback) => {
            context.external_ids.update_type = Some("message_callback".to_owned());
            context.external_ids.chat_id = payload_string(payload, "chat_id");
            context.external_ids.user_id = payload_string(payload, "user_id");
            context.external_ids.message_id = payload_string(payload, "message_id");
            context.external_ids.callback_id = payload_string(payload, "callback_id");

            Ok(Some(InboundEvent {
                event_name: "interaction.callback.received".to_owned(),
                context,
                payload: json!({
                    "callback_id": payload.get("callback_id"),
                    "message_id": payload.get("message_id"),
                    "payload": payload.get("payload"),
                    "timestamp": update.timestamp,
                    "raw": update.payload,
                }),
            }))
        }
        UpdateType::Known(_) | UpdateType::Unknown(_) => Ok(None),
    }
}
