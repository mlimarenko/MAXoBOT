//! Message read/list operations.

use http::Method;
use serde::Deserialize;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::message::Message,
};

#[derive(Debug, Deserialize)]
struct MessagesEnvelope {
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct MessageEnvelope {
    message: Message,
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Gets messages via `GET /messages`.
    pub async fn get_messages(&self) -> Result<Vec<Message>, ApiError> {
        let request = self
            .request_builder(Method::GET, api_paths::MESSAGES)?
            .build()?;
        let envelope: MessagesEnvelope = self.execute_json(request).await?;
        Ok(envelope.messages)
    }

    /// Gets message by ID via `GET /messages/{messageId}`.
    pub async fn get_message_by_id(&self, message_id: &str) -> Result<Message, ApiError> {
        let path = api_paths::message(message_id)
            .map_err(|error| ApiError::InvalidConfiguration(error.to_string()))?;
        let request = self.request_builder(Method::GET, path)?.build()?;
        let envelope: MessageEnvelope = self.execute_json(request).await?;
        Ok(envelope.message)
    }
}
