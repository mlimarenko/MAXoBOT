//! Callback answer operation.

use http::Method;
use serde::Serialize;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::{action_result::ActionResult, new_message_body::NewMessageBody},
};

/// Request body for callback answer endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct CallbackAnswerRequest {
    /// Optional message payload sent as callback response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<NewMessageBody>,
    /// Optional popup/notification text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notification: Option<String>,
}

impl CallbackAnswerRequest {
    /// Creates empty callback answer request.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets callback response message body.
    #[must_use]
    pub fn with_message(mut self, message: NewMessageBody) -> Self {
        self.message = Some(message);
        self
    }

    /// Sets callback notification text.
    #[must_use]
    pub fn with_notification(mut self, notification: impl Into<String>) -> Self {
        self.notification = Some(notification.into());
        self
    }

    fn validate(&self) -> Result<(), ApiError> {
        if self.message.is_none() && self.notification.is_none() {
            return Err(ApiError::InvalidConfiguration(
                "callback answer must include message or notification".to_owned(),
            ));
        }

        if let Some(message) = &self.message {
            message
                .validate()
                .map_err(|error| ApiError::InvalidConfiguration(error.to_string()))?;
        }

        if let Some(notification) = &self.notification
            && notification.trim().is_empty()
        {
            return Err(ApiError::InvalidConfiguration(
                "notification must not be empty when set".to_owned(),
            ));
        }

        Ok(())
    }
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Answers callback via `POST /answers?callback_id=...`.
    pub async fn answer_callback(
        &self,
        callback_id: &str,
        request: &CallbackAnswerRequest,
    ) -> Result<ActionResult, ApiError> {
        if callback_id.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "callback_id must not be empty".to_owned(),
            ));
        }
        request.validate()?;

        let request = self
            .request_builder(Method::POST, api_paths::ANSWERS)?
            .with_query_param("callback_id", callback_id)
            .with_body(request)?
            .build()?;

        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }
}
