//! Edit-message operation.

use http::Method;
use serde::Deserialize;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::{message::Message, new_message_body::NewMessageBody},
};

/// Request for `PUT /messages`.
#[derive(Debug, Clone)]
pub struct EditMessageRequest {
    /// Target message ID.
    pub message_id: String,
    /// New message body.
    pub body: NewMessageBody,
}

impl EditMessageRequest {
    /// Creates edit request.
    #[must_use]
    pub fn new(message_id: impl Into<String>, body: NewMessageBody) -> Self {
        Self {
            message_id: message_id.into(),
            body,
        }
    }

    fn validate(&self) -> Result<(), ApiError> {
        if self.message_id.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "message_id must not be empty".to_owned(),
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
    /// Edits message via `PUT /messages`.
    pub async fn edit_message(&self, request: &EditMessageRequest) -> Result<Message, ApiError> {
        request.validate()?;

        let request = self
            .request_builder(Method::PUT, api_paths::MESSAGES)?
            .with_query_param("message_id", &request.message_id)
            .with_body(&request.body)?
            .build()?;

        let envelope: MessageEnvelope = self.execute_json(request).await?;
        Ok(envelope.message)
    }
}
