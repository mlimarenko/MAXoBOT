//! User model primitives for MAX API payloads.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// MAX user object with forward-compatible extra-field capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct User {
    #[serde(default)]
    user_id: Option<i64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    is_bot: bool,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl User {
    /// Returns user identifier when available.
    pub fn id(&self) -> Option<i64> {
        self.user_id
    }

    /// Returns user display name.
    pub fn name(&self) -> Option<&str> {
        non_empty(self.name.as_deref())
    }

    /// Returns user username.
    pub fn username(&self) -> Option<&str> {
        non_empty(self.username.as_deref())
    }

    /// Returns `true` for bot users.
    pub fn is_bot(&self) -> bool {
        self.is_bot
    }

    /// Returns best effort display label: `name`, then `username`.
    pub fn display_name(&self) -> Option<&str> {
        self.name().or_else(|| self.username())
    }

    /// Returns unmodeled fields captured during deserialization.
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::User;

    #[test]
    fn parses_optional_fields() {
        let user: User =
            serde_json::from_str(r#"{"user_id":42,"is_bot":true}"#).expect("should parse user");

        assert_eq!(user.id(), Some(42));
        assert!(user.name().is_none());
        assert!(user.username().is_none());
        assert!(user.is_bot());
    }

    #[test]
    fn display_name_prefers_name_then_username() {
        let user_with_name: User =
            serde_json::from_str(r#"{"user_id":1,"name":"Alice","username":"alice"}"#)
                .expect("should parse");
        assert_eq!(user_with_name.display_name(), Some("Alice"));

        let user_with_username: User =
            serde_json::from_str(r#"{"user_id":1,"name":"  ","username":"alice"}"#)
                .expect("should parse");
        assert_eq!(user_with_username.display_name(), Some("alice"));
    }

    #[test]
    fn captures_extra_fields_for_forward_compatibility() {
        let user: User =
            serde_json::from_str(r#"{"user_id":1,"locale":"en"}"#).expect("should parse");

        assert_eq!(
            user.extra()
                .get("locale")
                .and_then(serde_json::Value::as_str),
            Some("en")
        );
    }
}
