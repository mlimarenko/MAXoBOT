use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use bytes::Bytes;
use http::StatusCode;
use reqwest::header::HeaderMap;
use serde_json::json;

use maxobot_core::{
    api::client::BotApiClient,
    auth::credentials::BotCredentials,
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    config::client_config::ClientConfig,
    errors::api_error::ApiError,
    updates::mode_guard::{PollingModeConflictPolicy, enforce_polling_mode_guard},
};

const TEST_TOKEN: &str = "mode-switch-test-token";

#[derive(Debug, Clone, Default)]
struct MockHttpExecutor {
    state: Arc<Mutex<MockExecutorState>>,
}

#[derive(Debug, Default)]
struct MockExecutorState {
    queued_responses: VecDeque<Result<HttpResponse, ApiError>>,
}

impl MockHttpExecutor {
    fn queue_json_response(&self, body: serde_json::Value) {
        let payload = serde_json::to_vec(&body).expect("response fixture should serialize");
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::from(payload),
        };
        self.state
            .lock()
            .expect("mock executor lock should not be poisoned")
            .queued_responses
            .push_back(Ok(response));
    }
}

#[async_trait]
impl HttpExecutor for MockHttpExecutor {
    async fn execute(&self, _request: HttpRequest) -> Result<HttpResponse, ApiError> {
        self.state
            .lock()
            .expect("mock executor lock should not be poisoned")
            .queued_responses
            .pop_front()
            .expect("queued response should exist")
    }
}

fn make_client(executor: MockHttpExecutor) -> BotApiClient<MockHttpExecutor> {
    let credentials = BotCredentials::new(TEST_TOKEN).expect("test token should be valid");
    BotApiClient::new(executor, ClientConfig::default(), credentials).expect("client should build")
}

#[tokio::test]
async fn mode_guard_warn_policy_allows_polling_with_warning() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(json!({
        "subscriptions": [{"url": "https://example.com/webhook"}]
    }));
    let client = make_client(executor);

    let result = enforce_polling_mode_guard(&client, PollingModeConflictPolicy::Warn)
        .await
        .expect("warn policy should allow polling");

    assert!(result.polling_allowed);
    assert_eq!(result.active_subscriptions, 1);
    assert!(result.warning.is_some());
    assert!(result.has_conflict());
}

#[tokio::test]
async fn mode_guard_fail_policy_rejects_polling_with_active_webhook_subscription() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(json!({
        "subscriptions": [{"url": "https://example.com/webhook"}]
    }));
    let client = make_client(executor);

    let result = enforce_polling_mode_guard(&client, PollingModeConflictPolicy::Fail).await;

    assert!(matches!(result, Err(ApiError::InvalidConfiguration(_))));
}
