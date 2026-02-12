//! Typed MAX API domain models.

/// Generic action result model for mutation endpoints.
pub mod action_result;

/// Attachment payload models used in inbound and outbound messages.
pub mod attachment;

/// Chat model primitives.
pub mod chat;

/// Chat membership model primitives.
pub mod chat_membership;

/// Message model primitives.
pub mod message;

/// Client-bound convenience helpers for message operations.
pub mod message_helpers;

/// Outbound message body model and validation helpers.
pub mod new_message_body;

/// User model primitives.
pub mod user;

/// Upload ticket model returned by `/uploads`.
pub mod upload_ticket;

/// Webhook subscription models.
pub mod subscription;

/// Video metadata model.
pub mod video;
