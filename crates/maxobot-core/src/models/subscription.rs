//! Webhook subscription models.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Webhook subscription descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WebhookSubscription {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    update_types: Vec<String>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl WebhookSubscription {
    /// Returns webhook URL.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        self.url
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns subscribed update types.
    #[must_use]
    pub fn update_types(&self) -> &[String] {
        &self.update_types
    }

    /// Returns forward-compatible extra fields.
    #[must_use]
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}
