//! Unmatched update reporting hooks.

use std::{future::Future, sync::Arc};

use async_trait::async_trait;
use maxobot_core::updates::update_envelope::UpdateEnvelope;

use crate::handler::DispatchContext;

/// Shared trait object for unmatched update handlers.
pub type SharedUnmatchedHandler = Arc<dyn UnmatchedUpdateHandler>;

/// Lightweight unmatched update report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmatchedUpdateEvent {
    /// Raw update type value.
    pub update_type: String,
    /// Update timestamp in Unix milliseconds.
    pub timestamp: i64,
    /// Trace ID from dispatch context.
    pub trace_id: uuid::Uuid,
}

/// Hook contract for updates that were not handled by any route.
#[async_trait]
pub trait UnmatchedUpdateHandler: Send + Sync {
    /// Called for every unmatched update.
    async fn on_unmatched(&self, update: &UpdateEnvelope, context: &DispatchContext);
}

#[async_trait]
impl<F, Fut> UnmatchedUpdateHandler for F
where
    F: Fn(UpdateEnvelope, DispatchContext) -> Fut + Send + Sync,
    Fut: Future<Output = ()> + Send,
{
    async fn on_unmatched(&self, update: &UpdateEnvelope, context: &DispatchContext) {
        (self)(update.clone(), context.clone()).await;
    }
}

/// Default unmatched handler that performs no action.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopUnmatchedHandler;

#[async_trait]
impl UnmatchedUpdateHandler for NoopUnmatchedHandler {
    async fn on_unmatched(&self, _update: &UpdateEnvelope, _context: &DispatchContext) {}
}

/// Creates unmatched event from update/context pair.
#[must_use]
pub fn to_event(update: &UpdateEnvelope, context: &DispatchContext) -> UnmatchedUpdateEvent {
    UnmatchedUpdateEvent {
        update_type: update.update_type.as_str().to_owned(),
        timestamp: update.timestamp,
        trace_id: context.trace_id,
    }
}

#[cfg(test)]
mod tests {
    use maxobot_core::updates::update_envelope::{UpdateEnvelope, UpdateSource, UpdateType};
    use serde_json::json;

    use super::to_event;
    use crate::handler::DispatchContext;

    #[test]
    fn event_contains_update_type_timestamp_and_trace() {
        let context = DispatchContext::default();
        let update = UpdateEnvelope {
            update_type: UpdateType::Unknown("future".to_owned()),
            timestamp: 1_700_000_333_000_i64,
            payload: json!({}),
            raw: json!({"update_type": "future"}),
            source: UpdateSource::Webhook,
        };
        let event = to_event(&update, &context);

        assert_eq!(event.update_type, "future");
        assert_eq!(event.timestamp, 1_700_000_333_000_i64);
        assert_eq!(event.trace_id, context.trace_id);
    }
}
