//! Chat mutation operations.

use http::Method;
use serde_json::Value;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::action_result::ActionResult,
};

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Patches chat via `PATCH /chats/{chatId}`.
    pub async fn patch_chat(&self, chat_id: i64, patch: &Value) -> Result<ActionResult, ApiError> {
        if !patch.is_object() {
            return Err(ApiError::InvalidConfiguration(
                "chat patch payload must be a JSON object".to_owned(),
            ));
        }

        let request = self
            .request_builder(Method::PATCH, api_paths::chat(chat_id))?
            .with_body(patch)?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }

    /// Deletes chat via `DELETE /chats/{chatId}`.
    pub async fn delete_chat(&self, chat_id: i64) -> Result<ActionResult, ApiError> {
        let request = self
            .request_builder(Method::DELETE, api_paths::chat(chat_id))?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }
}
