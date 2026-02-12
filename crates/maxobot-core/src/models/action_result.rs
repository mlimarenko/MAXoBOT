//! Generic action result model returned by mutation endpoints.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Generic operation result for side-effect endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ActionResult {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    message: Option<String>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl ActionResult {
    /// Creates a successful action result.
    #[must_use]
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            extra: Map::new(),
        }
    }

    /// Creates a failed action result with optional message.
    #[must_use]
    pub fn failure(message: Option<String>) -> Self {
        Self {
            success: false,
            message,
            extra: Map::new(),
        }
    }

    /// Returns whether operation succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Returns optional textual message.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns forward-compatible extra fields.
    #[must_use]
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}

#[cfg(test)]
mod tests {
    use super::ActionResult;

    #[test]
    fn success_constructor_sets_success_flag() {
        let result = ActionResult::success();
        assert!(result.is_success());
        assert!(result.message().is_none());
    }
}
