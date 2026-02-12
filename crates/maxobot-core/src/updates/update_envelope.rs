//! Typed inbound update envelope.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Source of update delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateSource {
    /// Long-polling source.
    Polling,
    /// Webhook source.
    Webhook,
}

/// Known update variants from MAX public docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownUpdateType {
    /// New message.
    MessageCreated,
    /// Callback button click.
    MessageCallback,
    /// Message edit.
    MessageEdited,
    /// Message remove.
    MessageRemoved,
    /// Bot added to chat.
    BotAdded,
    /// Bot removed from chat.
    BotRemoved,
    /// Dialog muted.
    DialogMuted,
    /// Dialog unmuted.
    DialogUnmuted,
    /// Dialog cleared.
    DialogCleared,
    /// Dialog removed.
    DialogRemoved,
    /// User added.
    UserAdded,
    /// User removed.
    UserRemoved,
    /// Bot started.
    BotStarted,
    /// Bot stopped.
    BotStopped,
    /// Chat title changed.
    ChatTitleChanged,
}

impl KnownUpdateType {
    /// Returns the canonical string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MessageCreated => "message_created",
            Self::MessageCallback => "message_callback",
            Self::MessageEdited => "message_edited",
            Self::MessageRemoved => "message_removed",
            Self::BotAdded => "bot_added",
            Self::BotRemoved => "bot_removed",
            Self::DialogMuted => "dialog_muted",
            Self::DialogUnmuted => "dialog_unmuted",
            Self::DialogCleared => "dialog_cleared",
            Self::DialogRemoved => "dialog_removed",
            Self::UserAdded => "user_added",
            Self::UserRemoved => "user_removed",
            Self::BotStarted => "bot_started",
            Self::BotStopped => "bot_stopped",
            Self::ChatTitleChanged => "chat_title_changed",
        }
    }

    /// Parses known update type by raw value.
    #[must_use]
    pub fn from_raw(raw: &str) -> Option<Self> {
        match raw {
            "message_created" => Some(Self::MessageCreated),
            "message_callback" => Some(Self::MessageCallback),
            "message_edited" => Some(Self::MessageEdited),
            "message_removed" => Some(Self::MessageRemoved),
            "bot_added" => Some(Self::BotAdded),
            "bot_removed" => Some(Self::BotRemoved),
            "dialog_muted" => Some(Self::DialogMuted),
            "dialog_unmuted" => Some(Self::DialogUnmuted),
            "dialog_cleared" => Some(Self::DialogCleared),
            "dialog_removed" => Some(Self::DialogRemoved),
            "user_added" => Some(Self::UserAdded),
            "user_removed" => Some(Self::UserRemoved),
            "bot_started" => Some(Self::BotStarted),
            "bot_stopped" => Some(Self::BotStopped),
            "chat_title_changed" => Some(Self::ChatTitleChanged),
            _ => None,
        }
    }
}

/// Update type with forward-compatible unknown variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UpdateType {
    /// Known update type.
    Known(KnownUpdateType),
    /// Unknown update type surfaced for safe handling.
    Unknown(String),
}

impl UpdateType {
    /// Parses type from raw string.
    #[must_use]
    pub fn from_raw(raw: &str) -> Self {
        match KnownUpdateType::from_raw(raw) {
            Some(value) => Self::Known(value),
            None => Self::Unknown(raw.to_owned()),
        }
    }

    /// Returns string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(value) => value.as_str(),
            Self::Unknown(value) => value.as_str(),
        }
    }
}

/// Normalized envelope for inbound updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateEnvelope {
    /// Parsed update type.
    pub update_type: UpdateType,
    /// Event timestamp in Unix milliseconds.
    pub timestamp: i64,
    /// Payload without envelope metadata.
    pub payload: Value,
    /// Raw envelope JSON.
    pub raw: Value,
    /// Ingestion source.
    pub source: UpdateSource,
}
