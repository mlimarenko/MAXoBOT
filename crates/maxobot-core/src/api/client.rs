//! Shared typed API client and request/response execution helpers.

use bytes::Bytes;
use http::Method;
use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;

use crate::{
    auth::{authorization::inject_authorization_header, credentials::BotCredentials},
    client::{
        http_executor::{HttpExecutor, HttpRequest},
        request_builder::RequestBuilder,
    },
    config::client_config::{ClientConfig, ClientConfigValidationError},
    errors::api_error::{ApiError, redact_sensitive},
};

/// Typed MAX API client parameterized by HTTP executor implementation.
#[derive(Debug, Clone)]
pub struct BotApiClient<E>
where
    E: HttpExecutor,
{
    executor: E,
    config: ClientConfig,
    credentials: BotCredentials,
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Creates a new typed API client with validated config and credentials.
    pub fn new(
        executor: E,
        config: ClientConfig,
        credentials: BotCredentials,
    ) -> Result<Self, ApiError> {
        config
            .validate()
            .map_err(client_config_error_to_api_error)?;
        credentials
            .validate()
            .map_err(|error| ApiError::InvalidConfiguration(error.to_string()))?;

        Ok(Self {
            executor,
            config,
            credentials,
        })
    }

    /// Returns runtime configuration.
    #[must_use]
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Returns authenticated bot credentials.
    #[must_use]
    pub fn credentials(&self) -> &BotCredentials {
        &self.credentials
    }

    /// Creates a preconfigured request builder with auth and user-agent headers.
    pub(crate) fn request_builder(
        &self,
        method: Method,
        path: impl Into<String>,
    ) -> Result<RequestBuilder, ApiError> {
        inject_authorization_header(
            &mut reqwest::header::HeaderMap::new(),
            self.credentials.token(),
        )?;

        RequestBuilder::new(self.config.api_base_url.clone(), method, path)
            .with_header(reqwest::header::AUTHORIZATION, self.credentials.token())?
            .with_header(USER_AGENT, &self.config.user_agent)
    }

    /// Executes request and deserializes JSON body as `T`.
    pub(crate) async fn execute_json<T>(&self, request: HttpRequest) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let response = self.executor.execute(request).await?;
        decode_json_body(&response.body)
    }

    /// Executes request and deserializes optional JSON body.
    pub(crate) async fn execute_optional_json<T>(
        &self,
        request: HttpRequest,
    ) -> Result<Option<T>, ApiError>
    where
        T: DeserializeOwned,
    {
        let response = self.executor.execute(request).await?;
        if response.body.is_empty() {
            return Ok(None);
        }

        decode_json_body(&response.body).map(Some)
    }
}

fn decode_json_body<T>(body: &Bytes) -> Result<T, ApiError>
where
    T: DeserializeOwned,
{
    serde_json::from_slice::<T>(body).map_err(|source| ApiError::ResponseDecode {
        source,
        body_preview: body_preview(body),
    })
}

fn body_preview(body: &Bytes) -> String {
    let utf8 = String::from_utf8_lossy(body);
    let preview: String = utf8.chars().take(256).collect();
    redact_sensitive(&preview)
}

fn client_config_error_to_api_error(error: ClientConfigValidationError) -> ApiError {
    ApiError::InvalidConfiguration(error.to_string())
}
