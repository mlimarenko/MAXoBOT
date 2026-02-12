//! Polling client for fetching updates pages with cursor integration.

use thiserror::Error;

use crate::{
    api::{
        client::BotApiClient,
        updates::{GetUpdatesRequest, UpdatesPage},
    },
    client::http_executor::HttpExecutor,
    errors::api_error::ApiError,
    updates::cursor_store::{CursorStore, CursorStoreError},
};

/// Polling client failures.
#[derive(Debug, Error)]
pub enum PollingClientError {
    /// API request failed.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// Cursor store operation failed.
    #[error(transparent)]
    CursorStore(#[from] CursorStoreError),
}

/// Updates polling client with cursor-store support.
#[derive(Debug)]
pub struct PollingClient<E, S>
where
    E: HttpExecutor,
    S: CursorStore,
{
    api_client: BotApiClient<E>,
    cursor_store: S,
}

impl<E, S> PollingClient<E, S>
where
    E: HttpExecutor,
    S: CursorStore,
{
    /// Creates polling client.
    #[must_use]
    pub fn new(api_client: BotApiClient<E>, cursor_store: S) -> Self {
        Self {
            api_client,
            cursor_store,
        }
    }

    /// Returns shared API client reference.
    #[must_use]
    pub fn api_client(&self) -> &BotApiClient<E> {
        &self.api_client
    }

    /// Returns cursor store reference.
    #[must_use]
    pub fn cursor_store(&self) -> &S {
        &self.cursor_store
    }

    /// Fetches next updates page, defaulting marker from store when missing.
    pub async fn fetch_updates(
        &self,
        request: &GetUpdatesRequest,
    ) -> Result<UpdatesPage, PollingClientError> {
        let mut resolved = request.clone();
        if resolved.marker.is_none() {
            resolved.marker = self.cursor_store.get_marker().await?;
        }

        let page = self.api_client.get_updates(&resolved).await?;
        self.cursor_store.set_marker(page.marker).await?;
        Ok(page)
    }

    /// Commits pending cursor marker.
    pub async fn commit_marker(&self) -> Result<Option<i64>, PollingClientError> {
        self.cursor_store.commit_marker().await.map_err(Into::into)
    }
}
