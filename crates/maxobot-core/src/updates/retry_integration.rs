//! Retry integration helpers for polling update workflows.

use crate::{
    api::updates::{GetUpdatesRequest, UpdatesPage},
    diagnostics::request_context::RequestContext,
    errors::api_error::ApiError,
    reliability::retry_executor::{RetryExecutor, RetryTelemetry},
    updates::{
        cursor_store::CursorStore,
        polling_client::{PollingClient, PollingClientError},
    },
};

/// Fetches updates using retry policy and telemetry callbacks.
pub async fn fetch_updates_with_retry<E, S, Telemetry>(
    polling_client: &PollingClient<E, S>,
    request: &GetUpdatesRequest,
    retry_executor: &RetryExecutor,
    telemetry: &Telemetry,
) -> Result<UpdatesPage, ApiError>
where
    E: crate::client::http_executor::HttpExecutor,
    S: CursorStore,
    Telemetry: RetryTelemetry,
{
    let request = request.clone();
    retry_executor
        .execute_with_telemetry(
            RequestContext::new("updates.fetch"),
            |_| async {
                polling_client
                    .fetch_updates(&request)
                    .await
                    .map_err(map_polling_error)
            },
            telemetry,
        )
        .await
}

/// Commits marker using retry policy and telemetry callbacks.
pub async fn commit_marker_with_retry<E, S, Telemetry>(
    polling_client: &PollingClient<E, S>,
    retry_executor: &RetryExecutor,
    telemetry: &Telemetry,
) -> Result<Option<i64>, ApiError>
where
    E: crate::client::http_executor::HttpExecutor,
    S: CursorStore,
    Telemetry: RetryTelemetry,
{
    retry_executor
        .execute_with_telemetry(
            RequestContext::new("updates.commit_marker"),
            |_| async {
                polling_client
                    .commit_marker()
                    .await
                    .map_err(map_polling_error)
            },
            telemetry,
        )
        .await
}

fn map_polling_error(error: PollingClientError) -> ApiError {
    match error {
        PollingClientError::Api(error) => error,
        PollingClientError::CursorStore(error) => {
            ApiError::InvalidConfiguration(format!("cursor store operation failed: {error}"))
        }
    }
}
