//! Long-polling updates API operation.

use http::Method;
use serde_json::Value;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    updates::{parser::parse_updates_page, update_envelope::UpdateEnvelope},
};

/// Query parameters for `GET /updates`.
#[derive(Debug, Clone, Default)]
pub struct GetUpdatesRequest {
    /// Max updates in page (`1..=1000`).
    pub limit: Option<u32>,
    /// Long-poll timeout seconds (`0..=90`).
    pub timeout: Option<u32>,
    /// Cursor marker.
    pub marker: Option<i64>,
    /// Optional update type filter list.
    pub types: Vec<String>,
}

impl GetUpdatesRequest {
    /// Creates empty request with defaults from server side.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates query options against contract ranges.
    pub fn validate(&self) -> Result<(), ApiError> {
        if let Some(limit) = self.limit
            && !(1..=1000).contains(&limit)
        {
            return Err(ApiError::InvalidConfiguration(
                "updates limit must be in range 1..=1000".to_owned(),
            ));
        }

        if let Some(timeout) = self.timeout
            && timeout > 90
        {
            return Err(ApiError::InvalidConfiguration(
                "updates timeout must be in range 0..=90".to_owned(),
            ));
        }

        Ok(())
    }
}

/// Decoded updates page.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdatesPage {
    /// Parsed update envelopes.
    pub updates: Vec<UpdateEnvelope>,
    /// Next marker for subsequent fetch.
    pub marker: Option<i64>,
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Fetches updates page via `GET /updates`.
    pub async fn get_updates(&self, request: &GetUpdatesRequest) -> Result<UpdatesPage, ApiError> {
        request.validate()?;

        let mut builder = self.request_builder(Method::GET, api_paths::UPDATES)?;
        if let Some(limit) = request.limit {
            builder = builder.with_query_param("limit", limit.to_string());
        }
        if let Some(timeout) = request.timeout {
            builder = builder.with_query_param("timeout", timeout.to_string());
        }
        if let Some(marker) = request.marker {
            builder = builder.with_query_param("marker", marker.to_string());
        }
        if !request.types.is_empty() {
            builder = builder.with_query_param("types", request.types.join(","));
        }

        let request = builder.build()?;
        let page: Value = self.execute_json(request).await?;
        let (updates, marker) = parse_updates_page(page)?;

        Ok(UpdatesPage { updates, marker })
    }
}
