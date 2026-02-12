//! Polling loop runtime with stop signal and graceful shutdown.

use std::future::Future;
use std::time::Duration;

use thiserror::Error;
use tokio::sync::watch;
use tokio::time::sleep;

use crate::{
    api::updates::GetUpdatesRequest,
    client::http_executor::HttpExecutor,
    reliability::retry_executor::{RetryExecutor, RetryTelemetry},
    updates::{
        commit_strategy::CommitStrategy,
        cursor_store::CursorStore,
        polling_client::{PollingClient, PollingClientError},
        retry_integration::{commit_marker_with_retry, fetch_updates_with_retry},
        update_envelope::UpdateEnvelope,
    },
};

/// Polling loop configuration.
#[derive(Debug, Clone, Copy)]
pub struct PollingLoopConfig {
    /// Pause between fetch iterations.
    pub poll_interval: Duration,
    /// Maximum updates handled per iteration (`0` means no limit).
    pub max_updates_per_tick: usize,
    /// Marker commit strategy.
    pub commit_strategy: CommitStrategy,
}

impl Default for PollingLoopConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(250),
            max_updates_per_tick: 0,
            commit_strategy: CommitStrategy::AfterSuccess,
        }
    }
}

/// Polling loop failures.
#[derive(Debug, Error)]
pub enum PollingLoopError {
    /// Fetch/commit failed.
    #[error(transparent)]
    PollingClient(#[from] PollingClientError),
    /// Update handler failed.
    #[error("update handler failed: {0}")]
    Handler(String),
}

/// Polling loop runtime.
#[derive(Debug)]
pub struct PollingLoop<E, S>
where
    E: HttpExecutor,
    S: CursorStore,
{
    polling_client: PollingClient<E, S>,
    config: PollingLoopConfig,
}

impl<E, S> PollingLoop<E, S>
where
    E: HttpExecutor,
    S: CursorStore,
{
    /// Creates polling loop runtime.
    #[must_use]
    pub fn new(polling_client: PollingClient<E, S>, config: PollingLoopConfig) -> Self {
        Self {
            polling_client,
            config,
        }
    }

    /// Runs polling loop until stop signal is set.
    pub async fn run<H, Fut>(
        &self,
        mut stop: watch::Receiver<bool>,
        mut handler: H,
    ) -> Result<(), PollingLoopError>
    where
        H: FnMut(UpdateEnvelope) -> Fut,
        Fut: Future<Output = Result<(), String>>,
    {
        loop {
            if *stop.borrow() {
                break;
            }

            let page = self
                .polling_client
                .fetch_updates(&GetUpdatesRequest::default())
                .await?;

            for (index, update) in page.updates.into_iter().enumerate() {
                if self.config.max_updates_per_tick > 0 && index >= self.config.max_updates_per_tick
                {
                    break;
                }

                if let Err(error) = handler(update).await {
                    return Err(PollingLoopError::Handler(error));
                }
            }

            if self.config.commit_strategy.should_commit(true) {
                let _ = self.polling_client.commit_marker().await?;
            }

            tokio::select! {
                _ = stop.changed() => {}
                () = sleep(self.config.poll_interval) => {}
            }
        }

        Ok(())
    }

    /// Runs polling loop using retry executor for fetch and commit operations.
    pub async fn run_with_retry<H, Fut, Telemetry>(
        &self,
        mut stop: watch::Receiver<bool>,
        retry_executor: &RetryExecutor,
        retry_telemetry: &Telemetry,
        mut handler: H,
    ) -> Result<(), PollingLoopError>
    where
        H: FnMut(UpdateEnvelope) -> Fut,
        Fut: Future<Output = Result<(), String>>,
        Telemetry: RetryTelemetry,
    {
        loop {
            if *stop.borrow() {
                break;
            }

            let page = fetch_updates_with_retry(
                &self.polling_client,
                &GetUpdatesRequest::default(),
                retry_executor,
                retry_telemetry,
            )
            .await
            .map_err(|error| PollingLoopError::PollingClient(PollingClientError::Api(error)))?;

            for (index, update) in page.updates.into_iter().enumerate() {
                if self.config.max_updates_per_tick > 0 && index >= self.config.max_updates_per_tick
                {
                    break;
                }

                if let Err(error) = handler(update).await {
                    return Err(PollingLoopError::Handler(error));
                }
            }

            if self.config.commit_strategy.should_commit(true) {
                let _ =
                    commit_marker_with_retry(&self.polling_client, retry_executor, retry_telemetry)
                        .await
                        .map_err(|error| {
                            PollingLoopError::PollingClient(PollingClientError::Api(error))
                        })?;
            }

            tokio::select! {
                _ = stop.changed() => {}
                () = sleep(self.config.poll_interval) => {}
            }
        }

        Ok(())
    }
}
