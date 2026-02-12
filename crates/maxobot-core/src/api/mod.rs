//! Typed clients for MAX Bot API method groups.

/// Bot method-group operations (`GET /me`).
pub mod bot;

/// Callback answer operations (`POST /answers`).
pub mod callback_answers;

/// Shared typed API client implementation.
pub mod client;

/// Chat listing/read operations (`GET /chats`, `GET /chats/{chatId}`).
pub mod chats_list;

/// Chat mutation operations (`PATCH/DELETE /chats/{chatId}`).
pub mod chats_manage;

/// Chat membership/admin operations.
pub mod chats_members;

/// Chat action and pin operations.
pub mod chats_actions;

/// Delete-message operation (`DELETE /messages`).
pub mod messages_delete;

/// Edit-message operation (`PUT /messages`).
pub mod messages_edit;

/// Read-message operations (`GET /messages`, `GET /messages/{messageId}`).
pub mod messages_read;

/// Send-message operation (`POST /messages`).
pub mod messages_send;

/// Webhook subscription operations (`/subscriptions`).
pub mod subscriptions;

/// Upload ticket operations (`POST /uploads`).
pub mod uploads;

/// Long-polling updates operation (`GET /updates`).
pub mod updates;

/// Video metadata operation (`GET /videos/{videoToken}`).
pub mod videos;

pub use client::BotApiClient;
