use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use http::{Method, StatusCode};
use reqwest::header::{AUTHORIZATION, HeaderMap};
use serde_json::{Value, json};
use tokio::sync::watch;

use maxobot_core::{
    api::client::BotApiClient,
    auth::credentials::BotCredentials,
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    config::client_config::ClientConfig,
    errors::api_error::ApiError,
    updates::{
        commit_strategy::CommitStrategy,
        in_memory_cursor_store::InMemoryCursorStore,
        polling_client::PollingClient,
        polling_loop::{PollingLoop, PollingLoopConfig},
        update_envelope::{UpdateEnvelope, UpdateType},
    },
};

const TEST_TOKEN: &str = "bot-token-test";
const UNKNOWN_UPDATE_TYPE: &str = "future_update_kind";
const UNKNOWN_UPDATE_TIMESTAMP: i64 = 1_700_000_123;

#[derive(Debug, Clone, Default)]
struct MockHttpExecutor {
    state: Arc<Mutex<MockExecutorState>>,
}

#[derive(Debug, Default)]
struct MockExecutorState {
    captured_requests: Vec<HttpRequest>,
    queued_responses: VecDeque<Result<HttpResponse, ApiError>>,
}

impl MockHttpExecutor {
    fn queue_json_response(&self, status: StatusCode, body: Value) {
        let serialized = serde_json::to_vec(&body).expect("response fixture should serialize");
        let response = HttpResponse {
            status,
            headers: HeaderMap::new(),
            body: Bytes::from(serialized),
        };

        self.state
            .lock()
            .expect("mock executor lock should not be poisoned")
            .queued_responses
            .push_back(Ok(response));
    }

    fn take_requests(&self) -> Vec<HttpRequest> {
        let mut state = self
            .state
            .lock()
            .expect("mock executor lock should not be poisoned");
        std::mem::take(&mut state.captured_requests)
    }
}

#[async_trait]
impl HttpExecutor for MockHttpExecutor {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ApiError> {
        let mut state = self
            .state
            .lock()
            .expect("mock executor lock should not be poisoned");
        state.captured_requests.push(request);

        state
            .queued_responses
            .pop_front()
            .expect("queued response should exist for request")
    }
}

#[derive(Debug, Default)]
struct DispatchProbe {
    known_hits: usize,
    unknown_hits: usize,
    unknown_types: Vec<String>,
    unknown_payloads: Vec<Value>,
    unknown_timestamps: Vec<i64>,
}

fn make_client(executor: MockHttpExecutor) -> BotApiClient<MockHttpExecutor> {
    let config = ClientConfig::default();
    let credentials = BotCredentials::new(TEST_TOKEN).expect("test token should be valid");
    BotApiClient::new(executor, config, credentials).expect("client should build")
}

fn assert_common_contract(request: &HttpRequest, method: Method, path: &str) {
    assert_eq!(request.method, method);
    assert_eq!(request.url.path(), path);

    let authorization = request
        .headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok());
    assert_eq!(authorization, Some(TEST_TOKEN));
}

fn handle_known_update(probe: &Arc<Mutex<DispatchProbe>>) {
    let mut guard = probe
        .lock()
        .expect("dispatch probe lock should not be poisoned");
    guard.known_hits += 1;
}

fn handle_unknown_update(
    probe: &Arc<Mutex<DispatchProbe>>,
    raw_type: &str,
    update: &UpdateEnvelope,
    stop_tx: &watch::Sender<bool>,
) -> Result<(), String> {
    {
        let mut guard = probe
            .lock()
            .expect("dispatch probe lock should not be poisoned");
        guard.unknown_hits += 1;
        guard.unknown_types.push(raw_type.to_owned());
        guard.unknown_payloads.push(update.payload.clone());
        guard.unknown_timestamps.push(update.timestamp);
    }

    stop_tx
        .send(true)
        .map_err(|error| format!("failed to stop polling loop: {error}"))
}

fn dispatch_update(
    update: UpdateEnvelope,
    probe: &Arc<Mutex<DispatchProbe>>,
    stop_tx: &watch::Sender<bool>,
) -> Result<(), String> {
    match &update.update_type {
        UpdateType::Known(_) => {
            handle_known_update(probe);
            Ok(())
        }
        UpdateType::Unknown(raw_type) => handle_unknown_update(probe, raw_type, &update, stop_tx),
    }
}

#[tokio::test]
async fn unknown_update_dispatch_routes_to_explicit_unknown_handler_path() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({
            "updates": [{
                "update_type": UNKNOWN_UPDATE_TYPE,
                "timestamp": UNKNOWN_UPDATE_TIMESTAMP,
                "payload": {
                    "message_id": "unknown-msg-1",
                    "chat_id": 777
                }
            }],
            "marker": 77
        }),
    );

    let client = make_client(executor.clone());
    let polling_client = PollingClient::new(client, InMemoryCursorStore::new());
    let polling_loop = PollingLoop::new(
        polling_client,
        PollingLoopConfig {
            poll_interval: Duration::from_millis(1),
            max_updates_per_tick: 0,
            commit_strategy: CommitStrategy::AfterSuccess,
        },
    );

    let (stop_tx, stop_rx) = watch::channel(false);
    let probe = Arc::new(Mutex::new(DispatchProbe::default()));

    polling_loop
        .run(stop_rx, {
            let probe = Arc::clone(&probe);
            let stop_tx = stop_tx.clone();
            move |update| {
                let probe = Arc::clone(&probe);
                let stop_tx = stop_tx.clone();
                async move { dispatch_update(update, &probe, &stop_tx) }
            }
        })
        .await
        .expect("polling loop should complete without runtime errors");

    let (known_hits, unknown_hits, unknown_types, unknown_timestamps, unknown_payloads) = {
        let guard = probe
            .lock()
            .expect("dispatch probe lock should not be poisoned");
        (
            guard.known_hits,
            guard.unknown_hits,
            guard.unknown_types.clone(),
            guard.unknown_timestamps.clone(),
            guard.unknown_payloads.clone(),
        )
    };

    assert_eq!(known_hits, 0, "known handler must not run");
    assert_eq!(unknown_hits, 1, "unknown handler must run once");
    assert_eq!(unknown_types, vec![UNKNOWN_UPDATE_TYPE.to_owned()]);
    assert_eq!(unknown_timestamps, vec![UNKNOWN_UPDATE_TIMESTAMP]);
    assert_eq!(
        unknown_payloads,
        vec![json!({
            "payload": {
                "message_id": "unknown-msg-1",
                "chat_id": 777
            }
        })]
    );

    let captured_requests = executor.take_requests();
    assert_eq!(
        captured_requests.len(),
        1,
        "one polling request is expected"
    );
    let request = &captured_requests[0];
    assert_common_contract(request, Method::GET, "/updates");
    assert!(
        request.query.is_empty(),
        "default polling request should not set query parameters"
    );
}
