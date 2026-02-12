//! Message models for MAX API responses and update payloads.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::models::{
    attachment::{Attachment, LinkMetadata},
    user::User,
};

/// Message entity returned by MAX API operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Message {
    #[serde(default)]
    message_id: Option<String>,
    #[serde(default)]
    sender: Option<User>,
    #[serde(default)]
    recipient: Option<MessageRecipient>,
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default)]
    body: Option<MessageBody>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl Message {
    /// Returns message identifier when available.
    pub fn id(&self) -> Option<&str> {
        non_empty(self.message_id.as_deref())
    }

    /// Returns message sender.
    pub fn sender(&self) -> Option<&User> {
        self.sender.as_ref()
    }

    /// Returns message recipient information.
    pub fn recipient(&self) -> Option<&MessageRecipient> {
        self.recipient.as_ref()
    }

    /// Returns UNIX timestamp from API payload when available.
    pub fn timestamp(&self) -> Option<i64> {
        self.timestamp
    }

    /// Returns body payload when available.
    pub fn body(&self) -> Option<&MessageBody> {
        self.body.as_ref()
    }

    /// Returns body payload classification.
    pub fn payload_kind(&self) -> MessagePayloadKind {
        self.body
            .as_ref()
            .map_or(MessagePayloadKind::Empty, MessageBody::payload_kind)
    }

    /// Returns unmodeled fields captured during deserialization.
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}

/// Message recipient coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MessageRecipient {
    #[serde(default)]
    chat_id: Option<i64>,
    #[serde(default)]
    user_id: Option<i64>,
}

impl MessageRecipient {
    /// Returns chat recipient identifier.
    pub fn chat_id(&self) -> Option<i64> {
        self.chat_id
    }

    /// Returns direct user recipient identifier.
    pub fn user_id(&self) -> Option<i64> {
        self.user_id
    }

    /// Returns `true` when recipient points to chat.
    pub fn is_chat(&self) -> bool {
        self.chat_id.is_some()
    }

    /// Returns `true` when recipient points to direct user.
    pub fn is_direct_user(&self) -> bool {
        self.user_id.is_some()
    }
}

/// Supported text formatting modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageTextFormat {
    /// Markdown syntax.
    Markdown,
    /// HTML syntax.
    Html,
}

/// Structured message body payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageBody {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    attachments: Vec<Attachment>,
    #[serde(default)]
    link: Option<LinkMetadata>,
    #[serde(default)]
    notify: Option<bool>,
    #[serde(default)]
    format: Option<MessageTextFormat>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl MessageBody {
    /// Returns body text.
    pub fn text(&self) -> Option<&str> {
        non_empty(self.text.as_deref())
    }

    /// Returns message attachments.
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }

    /// Returns link metadata when present.
    pub fn link(&self) -> Option<&LinkMetadata> {
        self.link.as_ref()
    }

    /// Returns explicit notify override if API provides it.
    pub fn notify(&self) -> Option<bool> {
        self.notify
    }

    /// Returns text formatting mode.
    pub fn format(&self) -> Option<MessageTextFormat> {
        self.format
    }

    /// Returns body payload classification.
    pub fn payload_kind(&self) -> MessagePayloadKind {
        let has_text = self.text().is_some();
        let has_attachments = !self.attachments.is_empty();
        let has_link = self.link.is_some();

        match (has_text, has_attachments, has_link) {
            (false, false, false) => MessagePayloadKind::Empty,
            (true, false, false) => MessagePayloadKind::Text,
            (false, true, false) => MessagePayloadKind::Attachments,
            (false, false, true) => MessagePayloadKind::Link,
            _ => MessagePayloadKind::Rich,
        }
    }

    /// Returns unmodeled fields captured during deserialization.
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}

/// Message payload composition category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagePayloadKind {
    /// Body is empty.
    Empty,
    /// Body carries text only.
    Text,
    /// Body carries attachments only.
    Attachments,
    /// Body carries link metadata only.
    Link,
    /// Body contains multiple payload components.
    Rich,
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
    use super::{Message, MessagePayloadKind};

    #[test]
    fn parses_message_with_text_payload() {
        let message: Message = serde_json::from_str(
            r#"{"message_id":"m1","timestamp":100,"body":{"text":"hello","notify":true,"format":"markdown"}}"#,
        )
        .expect("should parse message");

        assert_eq!(message.id(), Some("m1"));
        assert_eq!(message.timestamp(), Some(100));
        assert_eq!(message.payload_kind(), MessagePayloadKind::Text);
    }

    #[test]
    fn payload_kind_detects_rich_payload() {
        let message: Message = serde_json::from_str(
            r#"{
                "message_id":"m2",
                "body":{
                    "text":"hello",
                    "attachments":[{"type":"image","payload":{"token":"t1"}}]
                }
            }"#,
        )
        .expect("should parse message");

        assert_eq!(message.payload_kind(), MessagePayloadKind::Rich);
    }

    #[test]
    fn captures_forward_compatible_fields() {
        let message: Message = serde_json::from_str(
            r#"{"message_id":"m1","delivery_state":"sent","body":{"text":"ok","custom":1}}"#,
        )
        .expect("should parse message");

        assert_eq!(
            message
                .extra()
                .get("delivery_state")
                .and_then(serde_json::Value::as_str),
            Some("sent")
        );
        assert_eq!(
            message
                .body()
                .and_then(|body| body.extra().get("custom"))
                .and_then(serde_json::Value::as_i64),
            Some(1)
        );
    }
}
