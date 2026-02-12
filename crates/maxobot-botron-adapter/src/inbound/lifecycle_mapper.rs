//! Inbound mapper for lifecycle and membership updates.

use serde_json::json;

use maxobot_core::updates::update_envelope::{KnownUpdateType, UpdateEnvelope, UpdateType};

use crate::{
    context::adapter_context::AdapterContext,
    errors::translator::{AdapterFailure, BotronFailureClass},
    inbound::{InboundEvent, payload_object, payload_string},
};

/// Maps lifecycle updates such as bot/user/chat events.
pub fn map_lifecycle_update(
    update: &UpdateEnvelope,
    mut context: AdapterContext,
) -> Result<Option<InboundEvent>, AdapterFailure> {
    let Some(payload) = payload_object(update) else {
        return Err(AdapterFailure::new(
            BotronFailureClass::ChannelContractError,
            "update payload is not an object",
        ));
    };

    let (event_name, update_type) = match &update.update_type {
        UpdateType::Known(KnownUpdateType::BotStarted) => {
            ("interaction.session.started", "bot_started")
        }
        UpdateType::Known(KnownUpdateType::BotStopped) => {
            ("interaction.session.stopped", "bot_stopped")
        }
        UpdateType::Known(KnownUpdateType::UserAdded | KnownUpdateType::UserRemoved) => (
            "interaction.participant.changed",
            update.update_type.as_str(),
        ),
        UpdateType::Known(KnownUpdateType::ChatTitleChanged) => {
            ("interaction.chat.updated", "chat_title_changed")
        }
        UpdateType::Known(_) | UpdateType::Unknown(_) => return Ok(None),
    };

    context.external_ids.update_type = Some(update_type.to_owned());
    context.external_ids.chat_id = payload_string(payload, "chat_id");
    context.external_ids.user_id = payload_string(payload, "user_id");
    context.external_ids.message_id = payload_string(payload, "message_id");

    Ok(Some(InboundEvent {
        event_name: event_name.to_owned(),
        context,
        payload: json!({
            "timestamp": update.timestamp,
            "chat_id": payload.get("chat_id"),
            "user_id": payload.get("user_id"),
            "actor_id": payload.get("actor_id"),
            "title": payload.get("title"),
            "raw": update.payload,
        }),
    }))
}
