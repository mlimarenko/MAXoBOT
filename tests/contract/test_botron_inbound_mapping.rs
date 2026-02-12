use maxobot_botron_adapter::{
    AdapterConfig, AdapterContext,
    inbound::map_inbound_update,
};
use maxobot_core::updates::update_envelope::{KnownUpdateType, UpdateEnvelope, UpdateSource, UpdateType};
use serde_json::json;

fn message_created_update() -> UpdateEnvelope {
    UpdateEnvelope {
        update_type: UpdateType::Known(KnownUpdateType::MessageCreated),
        timestamp: 1_700_000_000_001_i64,
        payload: json!({
            "payload": {
                "chat_id": 10,
                "user_id": 20,
                "message_id": "m-1",
                "text": "hello"
            }
        }),
        raw: json!({}),
        source: UpdateSource::Webhook,
    }
}

fn callback_update() -> UpdateEnvelope {
    UpdateEnvelope {
        update_type: UpdateType::Known(KnownUpdateType::MessageCallback),
        timestamp: 1_700_000_000_002_i64,
        payload: json!({
            "payload": {
                "chat_id": 10,
                "user_id": 20,
                "message_id": "m-1",
                "callback_id": "cb-1",
                "payload": {"button": "x"}
            }
        }),
        raw: json!({}),
        source: UpdateSource::Webhook,
    }
}

#[test]
fn maps_message_created_to_interaction_message_received_contract() {
    let event = map_inbound_update(
        &message_created_update(),
        AdapterContext::default(),
        &AdapterConfig::default(),
    )
    .expect("mapping should succeed")
    .expect("event should be mapped");

    assert_eq!(event.event_name, "interaction.message.received");
    assert_eq!(event.context.external_ids.chat_id.as_deref(), Some("10"));
    assert_eq!(event.context.external_ids.user_id.as_deref(), Some("20"));
    assert_eq!(event.context.external_ids.message_id.as_deref(), Some("m-1"));
    assert_eq!(event.payload["timestamp"], json!(1_700_000_000_001_i64));
}

#[test]
fn maps_message_callback_to_interaction_callback_received_contract() {
    let event = map_inbound_update(
        &callback_update(),
        AdapterContext::default(),
        &AdapterConfig::default(),
    )
    .expect("mapping should succeed")
    .expect("event should be mapped");

    assert_eq!(event.event_name, "interaction.callback.received");
    assert_eq!(event.context.external_ids.callback_id.as_deref(), Some("cb-1"));
    assert_eq!(event.payload["callback_id"], json!("cb-1"));
    assert_eq!(event.payload["message_id"], json!("m-1"));
}
