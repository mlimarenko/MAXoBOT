//! Deterministic sequential dispatcher.

use std::sync::Arc;

use maxobot_core::updates::update_envelope::UpdateEnvelope;
use tracing::debug;

use crate::{
    handler::{DispatchContext, DispatchError},
    middleware::MiddlewareChain,
    router::Router,
    unmatched::{NoopUnmatchedHandler, SharedUnmatchedHandler, UnmatchedUpdateHandler},
};

/// Dispatch execution summary for one update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchOutcome {
    /// Number of routes with selector match.
    pub matched_routes: usize,
    /// Number of handlers that were executed.
    pub executed_routes: usize,
    /// Number of routes skipped by filters.
    pub skipped_routes: usize,
    /// Whether update remained unmatched by executable routes.
    pub unmatched: bool,
}

/// Deterministic dispatcher executing handlers one-by-one.
#[derive(Clone)]
pub struct SequentialDispatcher {
    router: Router,
    middleware: MiddlewareChain,
    unmatched_handler: SharedUnmatchedHandler,
}

impl SequentialDispatcher {
    /// Creates dispatcher with default middleware chain and unmatched policy.
    #[must_use]
    pub fn new(router: Router) -> Self {
        Self {
            router,
            middleware: MiddlewareChain::default(),
            unmatched_handler: Arc::new(NoopUnmatchedHandler),
        }
    }

    /// Replaces middleware chain.
    #[must_use]
    pub fn with_middleware_chain(mut self, middleware: MiddlewareChain) -> Self {
        self.middleware = middleware;
        self
    }

    /// Replaces unmatched update hook.
    #[must_use]
    pub fn with_unmatched_handler<H>(mut self, unmatched_handler: H) -> Self
    where
        H: UnmatchedUpdateHandler + 'static,
    {
        self.unmatched_handler = Arc::new(unmatched_handler);
        self
    }

    /// Returns router reference.
    #[must_use]
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Dispatches one update with a fresh context.
    pub async fn dispatch(&self, update: UpdateEnvelope) -> Result<DispatchOutcome, DispatchError> {
        self.dispatch_with_context(update, DispatchContext::default())
            .await
    }

    /// Dispatches one update with explicit context.
    pub async fn dispatch_with_context(
        &self,
        update: UpdateEnvelope,
        context: DispatchContext,
    ) -> Result<DispatchOutcome, DispatchError> {
        let mut matched_routes = 0usize;
        let mut executed_routes = 0usize;
        let mut skipped_routes = 0usize;

        for route in self.router.routes_by_priority() {
            if !route.matches_selector(&update) {
                continue;
            }
            matched_routes = matched_routes.saturating_add(1);

            if !route.passes_filters(&update, &context) {
                skipped_routes = skipped_routes.saturating_add(1);
                debug!(
                    route_id = route.id(),
                    update_type = update.update_type.as_str(),
                    "route skipped by filter",
                );
                continue;
            }

            let handler = Arc::clone(route.handler());
            self.middleware
                .execute(&update, &context, || handler.handle(&update, &context))
                .await?;
            executed_routes = executed_routes.saturating_add(1);
        }

        let unmatched = executed_routes == 0;
        if unmatched {
            self.unmatched_handler.on_unmatched(&update, &context).await;
        }

        Ok(DispatchOutcome {
            matched_routes,
            executed_routes,
            skipped_routes,
            unmatched,
        })
    }
}

impl std::fmt::Debug for SequentialDispatcher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SequentialDispatcher")
            .field("router", &self.router)
            .field("middleware", &self.middleware)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use serde_json::json;

    use super::SequentialDispatcher;
    use crate::{
        filter::by_source,
        handler::DispatchError,
        router::{Router, UpdateSelector, shared_handler},
    };
    use maxobot_core::updates::update_envelope::{
        KnownUpdateType, UpdateEnvelope, UpdateSource, UpdateType,
    };

    fn known_update(source: UpdateSource) -> UpdateEnvelope {
        UpdateEnvelope {
            update_type: UpdateType::Known(KnownUpdateType::MessageCreated),
            timestamp: 1_700_000_000_000_i64,
            payload: json!({"payload": {"id": 1}}),
            raw: json!({"update_type": "message_created"}),
            source,
        }
    }

    #[tokio::test]
    async fn sequential_dispatch_runs_handlers_in_priority_order() {
        let trace = Arc::new(AtomicUsize::new(0));
        let mut router = Router::new();

        let low_order = Arc::clone(&trace);
        router.register_with_priority(
            UpdateSelector::Any,
            1,
            shared_handler(move |_, _| {
                let low_order = Arc::clone(&low_order);
                async move {
                    assert_eq!(low_order.fetch_add(1, Ordering::SeqCst), 1);
                    Ok(())
                }
            }),
        );

        let high_order = Arc::clone(&trace);
        router.register_with_priority(
            UpdateSelector::Any,
            10,
            shared_handler(move |_, _| {
                let high_order = Arc::clone(&high_order);
                async move {
                    assert_eq!(high_order.fetch_add(1, Ordering::SeqCst), 0);
                    Ok(())
                }
            }),
        );

        let dispatcher = SequentialDispatcher::new(router);
        let outcome = dispatcher
            .dispatch(known_update(UpdateSource::Polling))
            .await
            .expect("dispatch should succeed");

        assert_eq!(trace.load(Ordering::SeqCst), 2);
        assert_eq!(outcome.executed_routes, 2);
        assert_eq!(outcome.skipped_routes, 0);
        assert!(!outcome.unmatched);
    }

    #[tokio::test]
    async fn sequential_dispatch_reports_filter_skips() {
        let mut router = Router::new();
        router.register_with_filters(
            UpdateSelector::Known(KnownUpdateType::MessageCreated),
            1,
            vec![by_source(UpdateSource::Webhook)],
            shared_handler(|_, _| async { Ok(()) }),
        );
        let dispatcher = SequentialDispatcher::new(router);
        let outcome = dispatcher
            .dispatch(known_update(UpdateSource::Polling))
            .await
            .expect("dispatch should succeed");

        assert_eq!(outcome.matched_routes, 1);
        assert_eq!(outcome.executed_routes, 0);
        assert_eq!(outcome.skipped_routes, 1);
        assert!(outcome.unmatched);
    }

    #[tokio::test]
    async fn sequential_dispatch_bubbles_handler_errors() {
        let mut router = Router::new();
        router.register(
            UpdateSelector::Any,
            shared_handler(|_, _| async { Err(DispatchError::Handler("boom".to_owned())) }),
        );
        let dispatcher = SequentialDispatcher::new(router);
        let result = dispatcher
            .dispatch(known_update(UpdateSource::Webhook))
            .await;

        assert_eq!(result, Err(DispatchError::Handler("boom".to_owned())));
    }
}
