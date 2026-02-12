//! Middleware contracts and chain execution orchestration.

use std::{future::Future, sync::Arc};

use async_trait::async_trait;
use maxobot_core::updates::update_envelope::UpdateEnvelope;

use crate::handler::{DispatchContext, DispatchResult, SharedUpdateHandler};

/// Shared trait object for dispatch middleware.
pub type SharedDispatchMiddleware = Arc<dyn DispatchMiddleware>;

/// Middleware contract with before/after hooks.
#[async_trait]
pub trait DispatchMiddleware: Send + Sync {
    /// Runs before route handler invocation.
    async fn before(&self, _update: &UpdateEnvelope, _context: &DispatchContext) -> DispatchResult {
        Ok(())
    }

    /// Runs after route handler invocation.
    async fn after(
        &self,
        _update: &UpdateEnvelope,
        _context: &DispatchContext,
        _result: &DispatchResult,
    ) -> DispatchResult {
        Ok(())
    }
}

/// Ordered middleware pipeline.
#[derive(Default, Clone)]
pub struct MiddlewareChain {
    middlewares: Vec<SharedDispatchMiddleware>,
}

impl MiddlewareChain {
    /// Creates empty chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds chain from explicit middleware list.
    #[must_use]
    pub fn from_middlewares(middlewares: Vec<SharedDispatchMiddleware>) -> Self {
        Self { middlewares }
    }

    /// Appends one middleware entry.
    pub fn push(&mut self, middleware: SharedDispatchMiddleware) {
        self.middlewares.push(middleware);
    }

    /// Returns middleware count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Returns whether chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// Runs middleware around one route handler.
    pub async fn execute<H, Fut>(
        &self,
        update: &UpdateEnvelope,
        context: &DispatchContext,
        handler: H,
    ) -> DispatchResult
    where
        H: FnOnce() -> Fut,
        Fut: Future<Output = DispatchResult>,
    {
        for middleware in &self.middlewares {
            middleware.before(update, context).await?;
        }

        let mut result = handler().await;

        for middleware in self.middlewares.iter().rev() {
            if let Err(error) = middleware.after(update, context, &result).await {
                if result.is_ok() {
                    result = Err(error);
                }
            }
        }

        result
    }

    /// Runs middleware around a shared handler trait object.
    pub async fn execute_handler(
        &self,
        update: &UpdateEnvelope,
        context: &DispatchContext,
        handler: &SharedUpdateHandler,
    ) -> DispatchResult {
        self.execute(update, context, || handler.handle(update, context))
            .await
    }
}

impl std::fmt::Debug for MiddlewareChain {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MiddlewareChain")
            .field("middleware_count", &self.middlewares.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use async_trait::async_trait;
    use maxobot_core::updates::update_envelope::{UpdateEnvelope, UpdateSource, UpdateType};
    use serde_json::json;

    use super::{DispatchMiddleware, MiddlewareChain};
    use crate::handler::{DispatchContext, DispatchError};

    #[derive(Debug)]
    struct CounterMiddleware {
        before_hits: Arc<AtomicUsize>,
        after_hits: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl DispatchMiddleware for CounterMiddleware {
        async fn before(
            &self,
            _update: &UpdateEnvelope,
            _context: &DispatchContext,
        ) -> Result<(), DispatchError> {
            self.before_hits.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn after(
            &self,
            _update: &UpdateEnvelope,
            _context: &DispatchContext,
            _result: &Result<(), DispatchError>,
        ) -> Result<(), DispatchError> {
            self.after_hits.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn fixture_update() -> UpdateEnvelope {
        UpdateEnvelope {
            update_type: UpdateType::Unknown("future".to_owned()),
            timestamp: 1_700_000_000_000_i64,
            payload: json!({"payload": {"id": 1}}),
            raw: json!({"update_type": "future"}),
            source: UpdateSource::Webhook,
        }
    }

    #[tokio::test]
    async fn middleware_chain_runs_before_and_after_hooks() {
        let before_hits = Arc::new(AtomicUsize::new(0));
        let after_hits = Arc::new(AtomicUsize::new(0));
        let middleware = CounterMiddleware {
            before_hits: Arc::clone(&before_hits),
            after_hits: Arc::clone(&after_hits),
        };

        let chain = MiddlewareChain::from_middlewares(vec![Arc::new(middleware)]);
        let result = chain
            .execute(&fixture_update(), &DispatchContext::default(), || async {
                Ok(())
            })
            .await;

        assert_eq!(result, Ok(()));
        assert_eq!(before_hits.load(Ordering::SeqCst), 1);
        assert_eq!(after_hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn middleware_chain_preserves_handler_error_if_after_fails() {
        struct FailingAfter;

        #[async_trait]
        impl DispatchMiddleware for FailingAfter {
            async fn after(
                &self,
                _update: &UpdateEnvelope,
                _context: &DispatchContext,
                _result: &Result<(), DispatchError>,
            ) -> Result<(), DispatchError> {
                Err(DispatchError::Middleware("after failed".to_owned()))
            }
        }

        let chain = MiddlewareChain::from_middlewares(vec![Arc::new(FailingAfter)]);
        let result = chain
            .execute(&fixture_update(), &DispatchContext::default(), || async {
                Err(DispatchError::Handler("handler failed".to_owned()))
            })
            .await;

        assert_eq!(
            result,
            Err(DispatchError::Handler("handler failed".to_owned()))
        );
    }
}
