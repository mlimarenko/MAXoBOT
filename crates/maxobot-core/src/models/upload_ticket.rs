//! Upload ticket model returned by `/uploads`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Upload ticket describing where to upload media and how to reference it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UploadTicket {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    token: Option<String>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl UploadTicket {
    /// Returns upload URL if present.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        self.url
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns media token if present.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns whether token exists and is non-empty.
    #[must_use]
    pub fn has_token(&self) -> bool {
        self.token().is_some()
    }

    /// Returns forward-compatible extra fields.
    #[must_use]
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}

#[cfg(test)]
mod tests {
    use super::UploadTicket;

    #[test]
    fn parses_url_and_token() {
        let ticket: UploadTicket =
            serde_json::from_str(r#"{"url":"https://upload.example","token":"t"}"#)
                .expect("ticket should decode");

        assert_eq!(ticket.url(), Some("https://upload.example"));
        assert_eq!(ticket.token(), Some("t"));
        assert!(ticket.has_token());
    }
}
