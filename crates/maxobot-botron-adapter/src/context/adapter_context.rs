//! Adapter context and external identifier model.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// External IDs propagated from MAX payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ExternalIdentifiers {
    /// MAX chat identifier.
    pub chat_id: Option<String>,
    /// MAX user identifier.
    pub user_id: Option<String>,
    /// MAX message identifier.
    pub message_id: Option<String>,
    /// MAX callback identifier.
    pub callback_id: Option<String>,
    /// MAX update type string.
    pub update_type: Option<String>,
}

/// Context propagated with mapped inbound and outbound adapter actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterContext {
    /// Channel name used by Botron.
    pub channel: String,
    /// Correlation trace identifier.
    pub trace_id: Uuid,
    /// External IDs captured from channel payloads.
    pub external_ids: ExternalIdentifiers,
    /// Additional adapter metadata.
    pub metadata: BTreeMap<String, String>,
}

impl AdapterContext {
    /// Creates base context with generated trace identifier.
    #[must_use]
    pub fn new() -> Self {
        Self {
            channel: "max".to_owned(),
            trace_id: Uuid::now_v7(),
            external_ids: ExternalIdentifiers::default(),
            metadata: BTreeMap::new(),
        }
    }

    /// Adds or replaces metadata value.
    pub fn insert_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Returns clone with explicit external identifiers.
    #[must_use]
    pub fn with_external_ids(mut self, external_ids: ExternalIdentifiers) -> Self {
        self.external_ids = external_ids;
        self
    }
}

impl Default for AdapterContext {
    fn default() -> Self {
        Self::new()
    }
}
