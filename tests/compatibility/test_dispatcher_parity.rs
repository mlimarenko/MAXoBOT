use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use maxobot_dispatch::{
    DispatchContext, DispatchError, DispatchMiddleware, MiddlewareChain, SequentialDispatcher,
    filter::by_source,
    router::{Router, UpdateSelector, shared_handler},
};
use maxobot_core::updates::update_envelope::{KnownUpdateType, UpdateEnvelope, UpdateSource, UpdateType};
use serde_json::json;

#[derive(Debug)]
struct TraceMiddleware {
    trace: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait]
impl DispatchMiddleware for TraceMiddleware {
    async fn before(
        &self,
        _update: &UpdateEnvelope,
        _context: &DispatchContext,
    ) -> Result<(), DispatchError> {
        self.trace
            .lock()
            .expect("lock should not be poisoned")
            .push("before");
        Ok(())
    }

    async fn after(
        &self,
        _update: &UpdateEnvelope,
        _context: &DispatchContext,
        _result: &Result<(), DispatchError>,
    ) -> Result<(), DispatchError> {
        self.trace
            .lock()
            .expect("lock should not be poisoned")
            .push("after");
        Ok(())
    }
}

#[tokio::test]
async fn dispatcher_filter_middleware_handler_order_matches_expected_parity_model() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let hits = Arc::new(AtomicUsize::new(0));
    let mut router = Router::new();

    router.register_with_filters(
        UpdateSelector::Known(KnownUpdateType::MessageCreated),
        10,
        vec![by_source(UpdateSource::Webhook)],
        shared_handler({
            let hits = Arc::clone(&hits);
            move |_, _| {
                let hits = Arc::clone(&hits);
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            }
        }),
    );

    let mut middleware = MiddlewareChain::default();
    middleware.push(Arc::new(TraceMiddleware {
        trace: Arc::clone(&trace),
    }));

    let dispatcher = SequentialDispatcher::new(router).with_middleware_chain(middleware);
    let update = UpdateEnvelope {
        update_type: UpdateType::Known(KnownUpdateType::MessageCreated),
        timestamp: 1_700_000_100_000_i64,
        payload: json!({"payload": {"chat_id": 1}}),
        raw: json!({}),
        source: UpdateSource::Webhook,
    };

    let outcome = dispatcher
        .dispatch(update)
        .await
        .expect("dispatch should succeed");

    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(*trace.lock().expect("lock should not be poisoned"), vec!["before", "after"]);
    assert_eq!(outcome.executed_routes, 1);
}
