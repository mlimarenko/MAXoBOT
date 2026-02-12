use maxobot_botron_adapter::{AdapterConfig, AdapterContext, UnknownEventPolicy, map_inbound_update};
use maxobot_core::updates::update_envelope::{UpdateEnvelope, UpdateSource, UpdateType};
use serde_json::json;

fn future_update() -> UpdateEnvelope {
    UpdateEnvelope {
        update_type: UpdateType::Unknown("brand_new_type".to_owned()),
        timestamp: 1_700_000_200_000_i64,
        payload: json!({"payload": {"field": "value"}}),
        raw: json!({"update_type": "brand_new_type"}),
        source: UpdateSource::Webhook,
    }
}

#[test]
fn unknown_future_update_maps_to_channel_unknown_by_default() {
    let event = map_inbound_update(&future_update(), AdapterContext::default(), &AdapterConfig::default())
        .expect("mapping should succeed")
        .expect("default policy should emit unknown event");

    assert_eq!(event.event_name, "interaction.channel.unknown");
}

#[test]
fn unknown_future_update_can_be_dropped_by_policy_without_runtime_failure() {
    let config = AdapterConfig {
        unknown_event_policy: UnknownEventPolicy::DropWithWarning,
        ..AdapterConfig::default()
    };

    let event = map_inbound_update(&future_update(), AdapterContext::default(), &config)
        .expect("drop policy should not fail");
    assert!(event.is_none());
}
