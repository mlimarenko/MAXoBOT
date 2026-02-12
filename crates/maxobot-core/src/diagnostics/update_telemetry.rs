//! Structured telemetry events for update ingestion lifecycle.

use serde_json::Value;
use uuid::Uuid;

use crate::{
    diagnostics::redaction::{RedactionConfig, redact_json},
    updates::update_envelope::UpdateEnvelope,
};

/// Event emitted after polling/webhook fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateFetchEvent {
    /// Correlation trace ID.
    pub trace_id: Uuid,
    /// Requested marker.
    pub requested_marker: Option<i64>,
    /// Returned marker.
    pub returned_marker: Option<i64>,
    /// Number of updates in page.
    pub update_count: usize,
}

/// Event emitted when update parsing completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateParseEvent {
    /// Correlation trace ID.
    pub trace_id: Uuid,
    /// Number of successfully parsed updates.
    pub parsed_count: usize,
    /// Number of parse failures.
    pub failed_count: usize,
}

/// Event emitted for one update dispatch attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateDispatchEvent {
    /// Correlation trace ID.
    pub trace_id: Uuid,
    /// Update type value.
    pub update_type: String,
    /// Whether update was handled by at least one route.
    pub handled: bool,
    /// Redacted payload snapshot.
    pub payload: Value,
}

/// Event emitted after marker commit attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCommitEvent {
    /// Correlation trace ID.
    pub trace_id: Uuid,
    /// Marker before commit.
    pub before_marker: Option<i64>,
    /// Marker after commit.
    pub committed_marker: Option<i64>,
    /// Whether commit was successful.
    pub committed: bool,
}

/// Telemetry callback hooks for update lifecycle.
pub trait UpdateTelemetry: Send + Sync {
    /// Called after updates fetch.
    fn on_fetch(&self, _event: &UpdateFetchEvent) {}

    /// Called after parse stage.
    fn on_parse(&self, _event: &UpdateParseEvent) {}

    /// Called for each dispatch operation.
    fn on_dispatch(&self, _event: &UpdateDispatchEvent) {}

    /// Called after marker commit stage.
    fn on_commit(&self, _event: &UpdateCommitEvent) {}
}

/// No-op telemetry implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopUpdateTelemetry;

impl UpdateTelemetry for NoopUpdateTelemetry {}

/// Builds redacted dispatch event from update payload.
#[must_use]
pub fn dispatch_event_from_update(
    trace_id: Uuid,
    update: &UpdateEnvelope,
    handled: bool,
    redaction: &RedactionConfig,
) -> UpdateDispatchEvent {
    UpdateDispatchEvent {
        trace_id,
        update_type: update.update_type.as_str().to_owned(),
        handled,
        payload: redact_json(&update.payload, redaction),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::dispatch_event_from_update;
    use crate::{
        diagnostics::redaction::RedactionConfig,
        updates::update_envelope::{UpdateEnvelope, UpdateSource, UpdateType},
    };

    #[test]
    fn dispatch_event_redacts_payload_fields() {
        let update = UpdateEnvelope {
            update_type: UpdateType::Unknown("future".to_owned()),
            timestamp: 1_700_000_100_000_i64,
            payload: json!({"token": "abc", "payload": {"text": "hello"}}),
            raw: json!({}),
            source: UpdateSource::Webhook,
        };
        let event =
            dispatch_event_from_update(Uuid::now_v7(), &update, true, &RedactionConfig::default());

        assert_eq!(event.update_type, "future");
        assert!(event.handled);
        assert_eq!(event.payload["token"], json!("***"));
    }
}
