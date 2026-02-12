//! Parser utilities for updates payloads.

use serde_json::Value;

use crate::errors::api_error::ApiError;
use crate::updates::update_envelope::{UpdateEnvelope, UpdateSource, UpdateType};

/// Parses a single update payload into a normalized envelope.
pub fn parse_update(raw: Value, source: UpdateSource) -> Result<UpdateEnvelope, ApiError> {
    let object = raw.as_object().ok_or_else(|| {
        ApiError::InvalidUpdatePayload("update payload must be object".to_owned())
    })?;

    let update_type_raw = object
        .get("update_type")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::InvalidUpdatePayload("missing `update_type` string".to_owned()))?;

    let timestamp = object
        .get("timestamp")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::InvalidUpdatePayload("missing `timestamp` int64".to_owned()))?;

    let mut payload = raw.clone();
    if let Some(payload_object) = payload.as_object_mut() {
        payload_object.remove("update_type");
        payload_object.remove("timestamp");
    }

    Ok(UpdateEnvelope {
        update_type: UpdateType::from_raw(update_type_raw),
        timestamp,
        payload,
        raw,
        source,
    })
}

/// Parses polling response page into update envelopes and next marker.
pub fn parse_updates_page(raw: Value) -> Result<(Vec<UpdateEnvelope>, Option<i64>), ApiError> {
    let object = raw.as_object().ok_or_else(|| {
        ApiError::InvalidUpdatePayload("updates page must be JSON object".to_owned())
    })?;

    let marker = match object.get("marker") {
        Some(Value::Number(value)) => value.as_i64(),
        Some(Value::Null) | None => None,
        Some(_) => {
            return Err(ApiError::InvalidUpdatePayload(
                "`marker` must be int64 or null".to_owned(),
            ));
        }
    };

    let updates = object
        .get("updates")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::InvalidUpdatePayload("missing `updates` array".to_owned()))?;

    let parsed = updates
        .iter()
        .cloned()
        .map(|update| parse_update(update, UpdateSource::Polling))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((parsed, marker))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::updates::update_envelope::UpdateType;

    use super::parse_update;

    #[test]
    fn keeps_unknown_update_type() {
        let raw = json!({
            "update_type": "new_type_from_future",
            "timestamp": 1234,
            "payload": {"x": 1}
        });

        let envelope = parse_update(raw, super::UpdateSource::Webhook).expect("parsed");
        assert!(matches!(envelope.update_type, UpdateType::Unknown(_)));
    }
}
