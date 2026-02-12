//! Chat model primitives for MAX API payloads.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// MAX chat object with optional fields and forward-compatible extra-field capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Chat {
    #[serde(default)]
    chat_id: Option<i64>,
    #[serde(rename = "type", default)]
    chat_type: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl Chat {
    /// Returns chat identifier when available.
    pub fn id(&self) -> Option<i64> {
        self.chat_id
    }

    /// Returns chat type string as received from API.
    pub fn chat_type(&self) -> Option<&str> {
        non_empty(self.chat_type.as_deref())
    }

    /// Returns chat title.
    pub fn title(&self) -> Option<&str> {
        non_empty(self.title.as_deref())
    }

    /// Returns public username/handle when available.
    pub fn username(&self) -> Option<&str> {
        non_empty(self.username.as_deref())
    }

    /// Returns best effort human label: `title`, then `username`.
    pub fn display_title(&self) -> Option<&str> {
        self.title().or_else(|| self.username())
    }

    /// Returns whether chat type equals `group`.
    pub fn is_group(&self) -> bool {
        self.chat_type()
            .is_some_and(|chat_type| chat_type.eq_ignore_ascii_case("group"))
    }

    /// Returns whether chat type equals `private`.
    pub fn is_private(&self) -> bool {
        self.chat_type()
            .is_some_and(|chat_type| chat_type.eq_ignore_ascii_case("private"))
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
    use super::Chat;

    #[test]
    fn parses_optional_fields() {
        let chat: Chat =
            serde_json::from_str(r#"{"chat_id":7,"type":"group"}"#).expect("should parse chat");

        assert_eq!(chat.id(), Some(7));
        assert_eq!(chat.chat_type(), Some("group"));
        assert!(chat.title().is_none());
        assert!(chat.is_group());
        assert!(!chat.is_private());
    }

    #[test]
    fn display_title_prefers_title_then_username() {
        let titled: Chat =
            serde_json::from_str(r#"{"chat_id":7,"title":"Core Team","username":"core"}"#)
                .expect("should parse");
        assert_eq!(titled.display_title(), Some("Core Team"));

        let with_username: Chat =
            serde_json::from_str(r#"{"chat_id":7,"title":" ","username":"core"}"#)
                .expect("should parse");
        assert_eq!(with_username.display_title(), Some("core"));
    }

    #[test]
    fn captures_extra_fields_for_forward_compatibility() {
        let chat: Chat =
            serde_json::from_str(r#"{"chat_id":7,"members_count":12}"#).expect("should parse");

        assert_eq!(
            chat.extra()
                .get("members_count")
                .and_then(serde_json::Value::as_i64),
            Some(12)
        );
    }
}
