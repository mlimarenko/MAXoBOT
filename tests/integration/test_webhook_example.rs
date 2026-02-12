use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::body::Body;
use http::{Request, StatusCode};
use maxobot_dispatch::{
    SequentialDispatcher,
    router::{Router, UpdateSelector, shared_handler},
};
use maxobot_webhook::{
    axum_adapter::webhook_router_with_dispatcher,
    verifier::{DEFAULT_SECRET_HEADER, WebhookVerifier},
};
use serde_json::json;
use tower::ServiceExt;

fn webhook_payload() -> String {
    json!({
        "update_type": "message_created",
        "timestamp": 1_700_000_000_123_i64,
        "payload": {
            "chat_id": 10,
            "text": "hello"
        }
    })
    .to_string()
}

fn build_dispatcher(counter: Arc<AtomicUsize>) -> SequentialDispatcher {
    let mut router = Router::new();
    router.register(
        UpdateSelector::Any,
        shared_handler(move |_, _| {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }),
    );
    SequentialDispatcher::new(router)
}

#[tokio::test]
async fn webhook_example_route_verifies_secret_and_dispatches_update() {
    let handled = Arc::new(AtomicUsize::new(0));
    let app = webhook_router_with_dispatcher(
        WebhookVerifier::new(Some("secret-value".to_owned())),
        Arc::new(build_dispatcher(Arc::clone(&handled))),
    );

    let request = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header(DEFAULT_SECRET_HEADER, "secret-value")
        .header("content-type", "application/json")
        .body(Body::from(webhook_payload()))
        .expect("request should build");

    let response = app.oneshot(request).await.expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(handled.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn webhook_example_route_rejects_invalid_secret() {
    let handled = Arc::new(AtomicUsize::new(0));
    let app = webhook_router_with_dispatcher(
        WebhookVerifier::new(Some("secret-value".to_owned())),
        Arc::new(build_dispatcher(Arc::clone(&handled))),
    );

    let request = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header(DEFAULT_SECRET_HEADER, "wrong-secret")
        .header("content-type", "application/json")
        .body(Body::from(webhook_payload()))
        .expect("request should build");

    let response = app.oneshot(request).await.expect("request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(handled.load(Ordering::SeqCst), 0);
}
