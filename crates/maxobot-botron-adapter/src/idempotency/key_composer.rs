//! Deterministic idempotency key composer.

use crate::context::adapter_context::ExternalIdentifiers;

/// Composes deterministic idempotency keys from channel/update identifiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyComposer;

impl KeyComposer {
    /// Composes inbound key from external IDs and flow scope.
    #[must_use]
    pub fn inbound_key(flow_scope: &str, external_ids: &ExternalIdentifiers) -> String {
        format!(
            "max:{flow_scope}:{}:{}:{}:{}",
            external_ids.update_type.as_deref().unwrap_or("unknown"),
            external_ids.chat_id.as_deref().unwrap_or("-"),
            external_ids.message_id.as_deref().unwrap_or("-"),
            external_ids.callback_id.as_deref().unwrap_or("-")
        )
    }

    /// Composes outbound key from action identifier and flow scope.
    #[must_use]
    pub fn outbound_key(flow_scope: &str, action: &str, external_id: &str) -> String {
        format!("max:{flow_scope}:{action}:{external_id}")
    }
}

#[cfg(test)]
mod tests {
    use crate::context::adapter_context::ExternalIdentifiers;

    use super::KeyComposer;

    #[test]
    fn composes_stable_inbound_and_outbound_keys() {
        let inbound = KeyComposer::inbound_key(
            "flow-1",
            &ExternalIdentifiers {
                chat_id: Some("10".to_owned()),
                user_id: Some("20".to_owned()),
                message_id: Some("30".to_owned()),
                callback_id: None,
                update_type: Some("message_created".to_owned()),
            },
        );
        let outbound = KeyComposer::outbound_key("flow-1", "send_message", "30");

        assert_eq!(inbound, "max:flow-1:message_created:10:30:-");
        assert_eq!(outbound, "max:flow-1:send_message:30");
    }
}
