//! Chat action and pin operations.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::{action_result::ActionResult, message::Message},
};

/// Request for `POST /chats/{chatId}/actions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SendActionRequest {
    /// Action type as documented by MAX API.
    pub action: String,
}

impl SendActionRequest {
    /// Creates action request and validates non-empty action.
    pub fn new(action: impl Into<String>) -> Result<Self, ApiError> {
        let action = action.into();
        if action.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "action must not be empty".to_owned(),
            ));
        }
        Ok(Self { action })
    }
}

/// Request for `PUT /chats/{chatId}/pin`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PinMessageRequest {
    /// Message ID to pin.
    pub message_id: String,
}

impl PinMessageRequest {
    /// Creates pin-message request and validates message ID.
    pub fn new(message_id: impl Into<String>) -> Result<Self, ApiError> {
        let message_id = message_id.into();
        if message_id.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "message_id must not be empty".to_owned(),
            ));
        }

        Ok(Self { message_id })
    }
}

#[derive(Debug, Deserialize)]
struct MessageEnvelope {
    message: Message,
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Sends chat action via `POST /chats/{chatId}/actions`.
    pub async fn send_action(
        &self,
        chat_id: i64,
        request: &SendActionRequest,
    ) -> Result<ActionResult, ApiError> {
        if request.action.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "action must not be empty".to_owned(),
            ));
        }

        let request = self
            .request_builder(Method::POST, api_paths::chat_actions(chat_id))?
            .with_body(request)?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }

    /// Gets pinned message via `GET /chats/{chatId}/pin`.
    pub async fn get_pin(&self, chat_id: i64) -> Result<Message, ApiError> {
        let request = self
            .request_builder(Method::GET, api_paths::chat_pin(chat_id))?
            .build()?;
        let envelope: MessageEnvelope = self.execute_json(request).await?;
        Ok(envelope.message)
    }

    /// Pins message via `PUT /chats/{chatId}/pin`.
    pub async fn pin_message(
        &self,
        chat_id: i64,
        request: &PinMessageRequest,
    ) -> Result<ActionResult, ApiError> {
        if request.message_id.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "message_id must not be empty".to_owned(),
            ));
        }

        let request = self
            .request_builder(Method::PUT, api_paths::chat_pin(chat_id))?
            .with_body(request)?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }

    /// Deletes pinned message via `DELETE /chats/{chatId}/pin`.
    pub async fn delete_pin(&self, chat_id: i64) -> Result<ActionResult, ApiError> {
        let request = self
            .request_builder(Method::DELETE, api_paths::chat_pin(chat_id))?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }
}
