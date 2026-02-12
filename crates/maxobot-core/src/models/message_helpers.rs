//! Convenience message helper context bound to a client.

use crate::{
    api::{
        client::BotApiClient, messages_edit::EditMessageRequest, messages_send::SendMessageRequest,
    },
    client::http_executor::HttpExecutor,
    errors::api_error::ApiError,
    models::{message::Message, new_message_body::NewMessageBody},
};

/// Client-bound helper for message-centric convenience operations.
#[derive(Debug)]
pub struct MessageContext<'a, E>
where
    E: HttpExecutor,
{
    client: &'a BotApiClient<E>,
    message_id: String,
    chat_id: Option<i64>,
    user_id: Option<i64>,
}

impl<'a, E> MessageContext<'a, E>
where
    E: HttpExecutor,
{
    /// Creates helper context from message model.
    pub fn from_message(client: &'a BotApiClient<E>, message: &Message) -> Result<Self, ApiError> {
        let message_id = message.id().ok_or_else(|| {
            ApiError::InvalidConfiguration("message id is required for helper context".to_owned())
        })?;

        let recipient = message.recipient().ok_or_else(|| {
            ApiError::InvalidConfiguration(
                "message recipient is required for helper context".to_owned(),
            )
        })?;

        Ok(Self {
            client,
            message_id: message_id.to_owned(),
            chat_id: recipient.chat_id(),
            user_id: recipient.user_id(),
        })
    }

    /// Sends answer to same recipient.
    pub async fn answer(&self, body: NewMessageBody) -> Result<Message, ApiError> {
        let request = if let Some(chat_id) = self.chat_id {
            SendMessageRequest::to_chat(chat_id, body)
        } else if let Some(user_id) = self.user_id {
            SendMessageRequest::to_user(user_id, body)
        } else {
            return Err(ApiError::InvalidConfiguration(
                "cannot answer message without chat_id or user_id".to_owned(),
            ));
        };

        self.client.send_message(&request).await
    }

    /// Sends reply to same recipient.
    pub async fn reply(&self, body: NewMessageBody) -> Result<Message, ApiError> {
        self.answer(body).await
    }

    /// Forwards message as text marker to another recipient.
    pub async fn forward(
        &self,
        target_chat_id: Option<i64>,
        target_user_id: Option<i64>,
    ) -> Result<Message, ApiError> {
        let mut forward_body = NewMessageBody::new();
        forward_body = forward_body.with_text(format!("Forwarded message {}", self.message_id));
        let request = match (target_chat_id, target_user_id) {
            (Some(chat_id), None) => SendMessageRequest::to_chat(chat_id, forward_body),
            (None, Some(user_id)) => SendMessageRequest::to_user(user_id, forward_body),
            _ => {
                return Err(ApiError::InvalidConfiguration(
                    "forward target must be set as chat_id xor user_id".to_owned(),
                ));
            }
        };

        self.client.send_message(&request).await
    }

    /// Edits current message.
    pub async fn edit(&self, body: NewMessageBody) -> Result<Message, ApiError> {
        let request = EditMessageRequest::new(self.message_id.clone(), body);
        self.client.edit_message(&request).await
    }
}

/// Extension trait for binding message helpers.
pub trait MessageExt {
    /// Binds this message to client and returns helper context.
    fn bind<'a, E>(
        &'a self,
        client: &'a BotApiClient<E>,
    ) -> Result<MessageContext<'a, E>, ApiError>
    where
        E: HttpExecutor;
}

impl MessageExt for Message {
    fn bind<'a, E>(&'a self, client: &'a BotApiClient<E>) -> Result<MessageContext<'a, E>, ApiError>
    where
        E: HttpExecutor,
    {
        MessageContext::from_message(client, self)
    }
}
