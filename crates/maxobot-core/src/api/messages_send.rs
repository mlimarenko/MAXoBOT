//! Send-message operation with recipient XOR validation.

use http::Method;
use serde::Deserialize;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::{message::Message, new_message_body::NewMessageBody},
};

/// Request for `POST /messages`.
#[derive(Debug, Clone)]
pub struct SendMessageRequest {
    /// Chat recipient ID.
    pub chat_id: Option<i64>,
    /// Direct-user recipient ID.
    pub user_id: Option<i64>,
    /// Outbound body payload.
    pub body: NewMessageBody,
    /// Optional link-preview suppression.
    pub disable_link_preview: Option<bool>,
}

impl SendMessageRequest {
    /// Creates a request targeting chat recipient.
    #[must_use]
    pub fn to_chat(chat_id: i64, body: NewMessageBody) -> Self {
        Self {
            chat_id: Some(chat_id),
            user_id: None,
            body,
            disable_link_preview: None,
        }
    }

    /// Creates a request targeting direct-user recipient.
    #[must_use]
    pub fn to_user(user_id: i64, body: NewMessageBody) -> Self {
        Self {
            chat_id: None,
            user_id: Some(user_id),
            body,
            disable_link_preview: None,
        }
    }

    /// Enables/disables link preview suppression flag.
    #[must_use]
    pub fn with_disable_link_preview(mut self, disable_link_preview: bool) -> Self {
        self.disable_link_preview = Some(disable_link_preview);
        self
    }

    fn validate(&self) -> Result<(), ApiError> {
        let recipient_count =
            usize::from(self.chat_id.is_some()) + usize::from(self.user_id.is_some());
        if recipient_count != 1 {
            return Err(ApiError::InvalidConfiguration(
                "exactly one recipient must be set: chat_id xor user_id".to_owned(),
            ));
        }

        self.body
            .validate()
            .map_err(|error| ApiError::InvalidConfiguration(error.to_string()))
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
    /// Sends message via `POST /messages`.
    pub async fn send_message(&self, request: &SendMessageRequest) -> Result<Message, ApiError> {
        request.validate()?;

        let mut builder = self.request_builder(Method::POST, api_paths::MESSAGES)?;
        if let Some(chat_id) = request.chat_id {
            builder = builder.with_query_param("chat_id", chat_id.to_string());
        }
        if let Some(user_id) = request.user_id {
            builder = builder.with_query_param("user_id", user_id.to_string());
        }
        if let Some(disable_link_preview) = request.disable_link_preview {
            builder =
                builder.with_query_param("disable_link_preview", disable_link_preview.to_string());
        }

        let request = builder.with_body(&request.body)?.build()?;
        let envelope: MessageEnvelope = self.execute_json(request).await?;
        Ok(envelope.message)
    }
}
