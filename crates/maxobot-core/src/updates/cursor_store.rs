//! Cursor persistence trait for long-polling marker management.

use async_trait::async_trait;
use thiserror::Error;

/// Cursor store failures.
#[derive(Debug, Error)]
pub enum CursorStoreError {
    /// Marker persistence operation failed.
    #[error("cursor store operation failed: {0}")]
    Operation(String),
}

/// Cursor store interface used by polling runtime.
#[async_trait]
pub trait CursorStore: Send + Sync {
    /// Returns committed marker.
    async fn get_marker(&self) -> Result<Option<i64>, CursorStoreError>;

    /// Stores next marker as pending value.
    async fn set_marker(&self, marker: Option<i64>) -> Result<(), CursorStoreError>;

    /// Commits pending marker and returns committed value.
    async fn commit_marker(&self) -> Result<Option<i64>, CursorStoreError>;
}
