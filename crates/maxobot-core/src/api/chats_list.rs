//! Read-only chat listing and lookup operations.

use http::Method;
use serde::Deserialize;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::chat::Chat,
};

#[derive(Debug, Deserialize)]
struct ChatsEnvelope {
    chats: Vec<Chat>,
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Gets available chats via `GET /chats`.
    pub async fn get_chats(&self) -> Result<Vec<Chat>, ApiError> {
        let request = self
            .request_builder(Method::GET, api_paths::CHATS)?
            .build()?;
        let envelope: ChatsEnvelope = self.execute_json(request).await?;
        Ok(envelope.chats)
    }

    /// Gets a chat by ID via `GET /chats/{chatId}`.
    pub async fn get_chat_by_id(&self, chat_id: i64) -> Result<Chat, ApiError> {
        let request = self
            .request_builder(Method::GET, api_paths::chat(chat_id))?
            .build()?;
        self.execute_json(request).await
    }
}
