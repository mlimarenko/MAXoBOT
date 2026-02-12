//! Delete-message operation.

use http::Method;

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
    /// Deletes message via `DELETE /messages`.
    pub async fn delete_message(&self, message_id: &str) -> Result<ActionResult, ApiError> {
        if message_id.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "message_id must not be empty".to_owned(),
            ));
        }

        let request = self
            .request_builder(Method::DELETE, api_paths::MESSAGES)?
            .with_query_param("message_id", message_id)
            .build()?;

        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }
}
