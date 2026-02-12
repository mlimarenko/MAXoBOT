//! Handler contracts and dispatch context models.

use std::{collections::BTreeMap, future::Future, sync::Arc};

use async_trait::async_trait;
use maxobot_core::updates::update_envelope::UpdateEnvelope;
use thiserror::Error;
use uuid::Uuid;

/// Result returned by update handlers.
pub type DispatchResult = Result<(), DispatchError>;

/// Failures emitted by dispatcher runtime, middleware, and handlers.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// Handler-specific failure.
    #[error("handler failed: {0}")]
    Handler(String),
    /// Middleware-specific failure.
    #[error("middleware failed: {0}")]
    Middleware(String),
    /// Dispatcher runtime error.
    #[error("dispatcher runtime failed: {0}")]
    Runtime(String),
}

/// Shared trait object for registered update handlers.
pub type SharedUpdateHandler = Arc<dyn UpdateHandler>;

/// Dispatch metadata available to handlers and middleware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchContext {
    /// Request trace identifier for correlation.
    pub trace_id: Uuid,
    /// Index of update in the processed batch.
    pub update_index: usize,
    /// Sanitized key-value tags.
    pub tags: BTreeMap<String, String>,
}

impl DispatchContext {
    /// Creates a fresh dispatch context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::now_v7(),
            update_index: 0,
            tags: BTreeMap::new(),
        }
    }

    /// Returns a copy with explicit update index.
    #[must_use]
    pub fn for_update_index(mut self, update_index: usize) -> Self {
        self.update_index = update_index;
        self
    }

    /// Returns a copy using an explicit trace identifier.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: Uuid) -> Self {
        self.trace_id = trace_id;
        self
    }

    /// Inserts a sanitized tag entry.
    pub fn insert_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = sanitize(&key.into(), 48);
        let value = sanitize(&value.into(), 128);
        self.tags.insert(key, value);
    }
}

impl Default for DispatchContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Async handler contract for one update.
#[async_trait]
pub trait UpdateHandler: Send + Sync {
    /// Handles one parsed update.
    async fn handle(&self, update: &UpdateEnvelope, context: &DispatchContext) -> DispatchResult;
}

#[async_trait]
impl<F, Fut> UpdateHandler for F
where
    F: Fn(UpdateEnvelope, DispatchContext) -> Fut + Send + Sync,
    Fut: Future<Output = DispatchResult> + Send,
{
    async fn handle(&self, update: &UpdateEnvelope, context: &DispatchContext) -> DispatchResult {
        (self)(update.clone(), context.clone()).await
    }
}

/// Handler with shared typed context binding.
#[derive(Clone)]
pub struct ContextBoundHandler<C, F> {
    shared_context: Arc<C>,
    handler: F,
}

impl<C, F> ContextBoundHandler<C, F> {
    /// Returns shared bound context.
    #[must_use]
    pub fn shared_context(&self) -> &Arc<C> {
        &self.shared_context
    }
}

impl<C, F> std::fmt::Debug for ContextBoundHandler<C, F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContextBoundHandler")
            .field("has_shared_context", &true)
            .finish_non_exhaustive()
    }
}

/// Binds typed context to a closure-based handler.
#[must_use]
pub fn bind_handler_context<C, F, Fut>(
    shared_context: Arc<C>,
    handler: F,
) -> ContextBoundHandler<C, F>
where
    C: Send + Sync + 'static,
    F: Fn(UpdateEnvelope, DispatchContext, Arc<C>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = DispatchResult> + Send + 'static,
{
    ContextBoundHandler {
        shared_context,
        handler,
    }
}

#[async_trait]
impl<C, F, Fut> UpdateHandler for ContextBoundHandler<C, F>
where
    C: Send + Sync + 'static,
    F: Fn(UpdateEnvelope, DispatchContext, Arc<C>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = DispatchResult> + Send + 'static,
{
    async fn handle(&self, update: &UpdateEnvelope, context: &DispatchContext) -> DispatchResult {
        (self.handler)(
            update.clone(),
            context.clone(),
            Arc::clone(&self.shared_context),
        )
        .await
    }
}

fn sanitize(value: &str, max_len: usize) -> String {
    value
        .replace(['\n', '\r', '\t'], " ")
        .chars()
        .take(max_len)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use maxobot_core::updates::update_envelope::{UpdateEnvelope, UpdateSource, UpdateType};
    use serde_json::json;

    use super::{DispatchContext, DispatchError, UpdateHandler, bind_handler_context};

    fn fixture_update() -> UpdateEnvelope {
        UpdateEnvelope {
            update_type: UpdateType::Unknown("future_event".to_owned()),
            timestamp: 1_700_000_000_000_i64,
            payload: json!({"payload": {"chat_id": 7}}),
            raw: json!({"update_type": "future_event", "payload": {"chat_id": 7}}),
            source: UpdateSource::Polling,
        }
    }

    #[tokio::test]
    async fn closure_handler_receives_cloned_update_and_context() {
        let calls = Arc::new(AtomicUsize::new(0));
        let handler = {
            let calls = Arc::clone(&calls);
            move |_update: UpdateEnvelope, context: DispatchContext| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    if context.update_index == 3 {
                        Ok(())
                    } else {
                        Err(DispatchError::Handler("wrong update index".to_owned()))
                    }
                }
            }
        };

        let update = fixture_update();
        let context = DispatchContext::new().for_update_index(3);
        let result = handler.handle(&update, &context).await;

        assert_eq!(result, Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn bind_handler_context_injects_shared_state() {
        let shared = Arc::new("max".to_owned());
        let handler = bind_handler_context(Arc::clone(&shared), |_, _, state| async move {
            if state.as_str() == "max" {
                Ok(())
            } else {
                Err(DispatchError::Handler("unexpected state".to_owned()))
            }
        });

        let update = fixture_update();
        let result = handler.handle(&update, &DispatchContext::default()).await;

        assert_eq!(result, Ok(()));
    }
}
