use maxobot_botron_adapter::{AdapterConfig, AdapterContext, map_inbound_update};
use maxobot_core::updates::update_envelope::{UpdateEnvelope, UpdateSource, UpdateType};
use serde_json::json;

#[test]
fn unknown_update_maps_to_non_breaking_channel_unknown_event() {
    let update = UpdateEnvelope {
        update_type: UpdateType::Unknown("future_update".to_owned()),
        timestamp: 1_700_000_000_003_i64,
        payload: json!({
            "payload": {
                "any_field": "value"
            }
        }),
        raw: json!({"update_type": "future_update"}),
        source: UpdateSource::Polling,
    };

    let event = map_inbound_update(&update, AdapterContext::default(), &AdapterConfig::default())
        .expect("unknown update should not fail")
        .expect("unknown update should map to explicit event");

    assert_eq!(event.event_name, "interaction.channel.unknown");
    assert_eq!(event.payload["raw_update_type"], json!("future_update"));
    assert_eq!(event.payload["timestamp"], json!(1_700_000_000_003_i64));
}
