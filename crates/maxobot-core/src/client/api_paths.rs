//! MAX Bot API path constants and composition helpers.

use thiserror::Error;

/// `/me` method-group path.
pub const ME: &str = "/me";

/// `/chats` method-group path.
pub const CHATS: &str = "/chats";

/// `/messages` method-group path.
pub const MESSAGES: &str = "/messages";

/// `/subscriptions` method-group path.
pub const SUBSCRIPTIONS: &str = "/subscriptions";

/// `/updates` method-group path.
pub const UPDATES: &str = "/updates";

/// `/uploads` method-group path.
pub const UPLOADS: &str = "/uploads";

/// `/videos` method-group path.
pub const VIDEOS: &str = "/videos";

/// `/answers` method-group path.
pub const ANSWERS: &str = "/answers";

/// Template for `/chats/{chatId}`.
pub const CHAT_BY_ID_TEMPLATE: &str = "/chats/{chatId}";

/// Template for `/chats/{chatId}/actions`.
pub const CHAT_ACTIONS_TEMPLATE: &str = "/chats/{chatId}/actions";

/// Template for `/chats/{chatId}/pin`.
pub const CHAT_PIN_TEMPLATE: &str = "/chats/{chatId}/pin";

/// Template for `/chats/{chatId}/members/me`.
pub const CHAT_MEMBERS_ME_TEMPLATE: &str = "/chats/{chatId}/members/me";

/// Template for `/chats/{chatId}/members/admins`.
pub const CHAT_MEMBERS_ADMINS_TEMPLATE: &str = "/chats/{chatId}/members/admins";

/// Template for `/chats/{chatId}/members/admins/{userId}`.
pub const CHAT_MEMBERS_ADMIN_BY_ID_TEMPLATE: &str = "/chats/{chatId}/members/admins/{userId}";

/// Template for `/chats/{chatId}/members`.
pub const CHAT_MEMBERS_TEMPLATE: &str = "/chats/{chatId}/members";

/// Template for `/messages/{messageId}`.
pub const MESSAGE_BY_ID_TEMPLATE: &str = "/messages/{messageId}";

/// Template for `/videos/{videoToken}`.
pub const VIDEO_BY_TOKEN_TEMPLATE: &str = "/videos/{videoToken}";

/// Errors for endpoint path composition helpers.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApiPathError {
    /// Dynamic path segment is empty.
    #[error("{segment_name} path segment must not be empty")]
    EmptySegment {
        /// Segment name that failed validation.
        segment_name: &'static str,
    },

    /// Dynamic path segment contains `/` and would corrupt final path structure.
    #[error("{segment_name} path segment must not contain '/'")]
    InvalidSegment {
        /// Segment name that failed validation.
        segment_name: &'static str,
    },
}

/// Composes `/chats/{chatId}`.
pub fn chat(chat_id: i64) -> String {
    format!("{CHATS}/{chat_id}")
}

/// Composes `/chats/{chatId}/actions`.
pub fn chat_actions(chat_id: i64) -> String {
    format!("{}/actions", chat(chat_id))
}

/// Composes `/chats/{chatId}/pin`.
pub fn chat_pin(chat_id: i64) -> String {
    format!("{}/pin", chat(chat_id))
}

/// Composes `/chats/{chatId}/members/me`.
pub fn chat_members_me(chat_id: i64) -> String {
    format!("{}/members/me", chat(chat_id))
}

/// Composes `/chats/{chatId}/members/admins`.
pub fn chat_members_admins(chat_id: i64) -> String {
    format!("{}/members/admins", chat(chat_id))
}

/// Composes `/chats/{chatId}/members/admins/{userId}`.
pub fn chat_members_admin(chat_id: i64, user_id: i64) -> String {
    format!("{}/members/admins/{user_id}", chat(chat_id))
}

/// Composes `/chats/{chatId}/members`.
pub fn chat_members(chat_id: i64) -> String {
    format!("{}/members", chat(chat_id))
}

/// Composes `/messages/{messageId}`.
pub fn message(message_id: &str) -> Result<String, ApiPathError> {
    let message_id = checked_segment(message_id, "message_id")?;
    Ok(format!("{MESSAGES}/{message_id}"))
}

/// Composes `/videos/{videoToken}`.
pub fn video(video_token: &str) -> Result<String, ApiPathError> {
    let video_token = checked_segment(video_token, "video_token")?;
    Ok(format!("{VIDEOS}/{video_token}"))
}

fn checked_segment<'a>(
    value: &'a str,
    segment_name: &'static str,
) -> Result<&'a str, ApiPathError> {
    if value.trim().is_empty() {
        return Err(ApiPathError::EmptySegment { segment_name });
    }

    if value.contains('/') {
        return Err(ApiPathError::InvalidSegment { segment_name });
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        CHAT_ACTIONS_TEMPLATE, CHAT_BY_ID_TEMPLATE, CHAT_MEMBERS_ADMIN_BY_ID_TEMPLATE,
        CHAT_MEMBERS_ADMINS_TEMPLATE, CHAT_MEMBERS_ME_TEMPLATE, CHAT_MEMBERS_TEMPLATE,
        CHAT_PIN_TEMPLATE, MESSAGE_BY_ID_TEMPLATE, MESSAGES, VIDEO_BY_TOKEN_TEMPLATE, chat,
        chat_actions, chat_members, chat_members_admin, chat_members_admins, chat_members_me,
        chat_pin, message, video,
    };

    #[test]
    fn chat_paths_compose_as_expected() {
        assert_eq!(chat(42), "/chats/42");
        assert_eq!(chat_actions(42), "/chats/42/actions");
        assert_eq!(chat_pin(42), "/chats/42/pin");
        assert_eq!(chat_members_me(42), "/chats/42/members/me");
        assert_eq!(chat_members_admins(42), "/chats/42/members/admins");
        assert_eq!(chat_members_admin(42, 10), "/chats/42/members/admins/10");
        assert_eq!(chat_members(42), "/chats/42/members");
    }

    #[test]
    fn dynamic_message_and_video_paths_validate_segments() {
        assert_eq!(
            message("msg_1").expect("valid message id"),
            "/messages/msg_1"
        );
        assert_eq!(video("v_1").expect("valid video token"), "/videos/v_1");

        assert!(message(" ").is_err());
        assert!(message("bad/id").is_err());
        assert!(video("bad/id").is_err());
    }

    #[test]
    fn templates_match_composition_contract() {
        assert_eq!(CHAT_BY_ID_TEMPLATE, "/chats/{chatId}");
        assert_eq!(CHAT_ACTIONS_TEMPLATE, "/chats/{chatId}/actions");
        assert_eq!(CHAT_PIN_TEMPLATE, "/chats/{chatId}/pin");
        assert_eq!(CHAT_MEMBERS_ME_TEMPLATE, "/chats/{chatId}/members/me");
        assert_eq!(
            CHAT_MEMBERS_ADMINS_TEMPLATE,
            "/chats/{chatId}/members/admins"
        );
        assert_eq!(
            CHAT_MEMBERS_ADMIN_BY_ID_TEMPLATE,
            "/chats/{chatId}/members/admins/{userId}"
        );
        assert_eq!(CHAT_MEMBERS_TEMPLATE, "/chats/{chatId}/members");
        assert_eq!(MESSAGE_BY_ID_TEMPLATE, "/messages/{messageId}");
        assert_eq!(VIDEO_BY_TOKEN_TEMPLATE, "/videos/{videoToken}");

        assert!(MESSAGES.starts_with('/'));
    }
}
