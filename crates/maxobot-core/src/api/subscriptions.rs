//! Webhook subscription operations.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::{
    api::client::BotApiClient,
    auth::credentials::{BotCredentials, CredentialsValidationError},
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::{action_result::ActionResult, subscription::WebhookSubscription},
};

/// Request for `POST /subscriptions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateSubscriptionRequest {
    /// Target webhook URL.
    pub url: String,
    /// Filtered update types.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub update_types: Vec<String>,
    /// Optional webhook secret.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub secret: Option<String>,
}

impl CreateSubscriptionRequest {
    /// Creates request with URL and optional update type filter.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            update_types: Vec::new(),
            secret: None,
        }
    }

    /// Sets update types.
    #[must_use]
    pub fn with_update_types(mut self, update_types: Vec<String>) -> Self {
        self.update_types = update_types;
        self
    }

    /// Sets webhook secret.
    pub fn with_secret(mut self, secret: impl Into<String>) -> Result<Self, ApiError> {
        let secret = secret.into();
        BotCredentials::validate_webhook_secret(Some(&secret))
            .map_err(secret_validation_to_api_error)?;
        self.secret = Some(secret);
        Ok(self)
    }

    fn validate(&self) -> Result<(), ApiError> {
        if self.url.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "subscription url must not be empty".to_owned(),
            ));
        }
        if let Some(secret) = &self.secret {
            BotCredentials::validate_webhook_secret(Some(secret))
                .map_err(secret_validation_to_api_error)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct SubscriptionsEnvelope {
    #[serde(default)]
    subscriptions: Vec<WebhookSubscription>,
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Gets subscriptions via `GET /subscriptions`.
    pub async fn get_subscriptions(&self) -> Result<Vec<WebhookSubscription>, ApiError> {
        let request = self
            .request_builder(Method::GET, api_paths::SUBSCRIPTIONS)?
            .build()?;
        let envelope: SubscriptionsEnvelope = self.execute_json(request).await?;
        Ok(envelope.subscriptions)
    }

    /// Creates webhook subscription via `POST /subscriptions`.
    pub async fn subscribe_webhook(
        &self,
        request: &CreateSubscriptionRequest,
    ) -> Result<ActionResult, ApiError> {
        request.validate()?;

        let request = self
            .request_builder(Method::POST, api_paths::SUBSCRIPTIONS)?
            .with_body(request)?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }

    /// Deletes webhook subscription via `DELETE /subscriptions?url=...`.
    pub async fn unsubscribe_webhook(&self, url: &str) -> Result<ActionResult, ApiError> {
        if url.trim().is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "subscription url must not be empty".to_owned(),
            ));
        }

        let request = self
            .request_builder(Method::DELETE, api_paths::SUBSCRIPTIONS)?
            .with_query_param("url", url)
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }
}

fn secret_validation_to_api_error(error: CredentialsValidationError) -> ApiError {
    ApiError::InvalidConfiguration(error.to_string())
}
