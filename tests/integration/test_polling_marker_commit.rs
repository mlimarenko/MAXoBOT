use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use http::StatusCode;
use reqwest::header::HeaderMap;
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
        cursor_store::{CursorStore, CursorStoreError},
        polling_client::PollingClient,
        polling_loop::{PollingLoop, PollingLoopConfig, PollingLoopError},
    },
};

const TEST_TOKEN: &str = "bot-token-test";

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
    fn queue_updates_page_response(&self, marker: Option<i64>, updates: Vec<Value>) {
        self.queue_json_response(
            StatusCode::OK,
            json!({
                "updates": updates,
                "marker": marker
            }),
        );
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CursorSnapshot {
    committed: Option<i64>,
    pending: Option<i64>,
    set_calls: usize,
    commit_calls: usize,
}

#[derive(Debug, Clone, Default)]
struct MockCursorStore {
    state: Arc<Mutex<MockCursorState>>,
}

#[derive(Debug, Default)]
struct MockCursorState {
    committed: Option<i64>,
    pending: Option<i64>,
    set_calls: usize,
    commit_calls: usize,
}

impl MockCursorStore {
    fn snapshot(&self) -> CursorSnapshot {
        let state = self
            .state
            .lock()
            .expect("cursor store lock should not be poisoned");
        CursorSnapshot {
            committed: state.committed,
            pending: state.pending,
            set_calls: state.set_calls,
            commit_calls: state.commit_calls,
        }
    }
}

#[async_trait]
impl CursorStore for MockCursorStore {
    async fn get_marker(&self) -> Result<Option<i64>, CursorStoreError> {
        Ok(self
            .state
            .lock()
            .expect("cursor store lock should not be poisoned")
            .committed)
    }

    async fn set_marker(&self, marker: Option<i64>) -> Result<(), CursorStoreError> {
        {
            let mut state = self
                .state
                .lock()
                .expect("cursor store lock should not be poisoned");
            state.pending = marker;
            state.set_calls += 1;
        }
        Ok(())
    }

    async fn commit_marker(&self) -> Result<Option<i64>, CursorStoreError> {
        let mut state = self
            .state
            .lock()
            .expect("cursor store lock should not be poisoned");
        state.commit_calls += 1;
        state.committed = state.pending;
        let committed = state.committed;
        drop(state);
        Ok(committed)
    }
}

fn make_polling_client(
    executor: MockHttpExecutor,
    cursor_store: MockCursorStore,
) -> PollingClient<MockHttpExecutor, MockCursorStore> {
    let config = ClientConfig::default();
    let credentials = BotCredentials::new(TEST_TOKEN).expect("test token should be valid");
    let api_client = BotApiClient::new(executor, config, credentials).expect("client should build");
    PollingClient::new(api_client, cursor_store)
}

fn make_update_fixture(timestamp: i64) -> Value {
    json!({
        "update_type": "message_created",
        "timestamp": timestamp,
        "payload": {
            "id": timestamp
        }
    })
}

fn query_value<'a>(request: &'a HttpRequest, key: &str) -> Option<&'a str> {
    request
        .query
        .iter()
        .find_map(|(query_key, query_value)| (query_key == key).then_some(query_value.as_str()))
}

#[tokio::test]
async fn polling_loop_commits_marker_after_successful_handler_execution() {
    let executor = MockHttpExecutor::default();
    executor.queue_updates_page_response(Some(777), vec![make_update_fixture(1_725_435_900_000)]);

    let cursor_store = MockCursorStore::default();
    let polling_client = make_polling_client(executor.clone(), cursor_store.clone());
    let polling_loop = PollingLoop::new(
        polling_client,
        PollingLoopConfig {
            poll_interval: Duration::from_millis(1),
            max_updates_per_tick: 0,
            commit_strategy: CommitStrategy::AfterSuccess,
        },
    );

    let handled_calls = Arc::new(AtomicUsize::new(0));
    let (stop_tx, stop_rx) = watch::channel(false);

    let result = polling_loop
        .run(stop_rx, {
            let handled_calls = Arc::clone(&handled_calls);
            move |_| {
                let handled_calls = Arc::clone(&handled_calls);
                let stop_tx = stop_tx.clone();
                async move {
                    handled_calls.fetch_add(1, Ordering::SeqCst);
                    stop_tx
                        .send(true)
                        .expect("stop signal receiver should still exist");
                    Ok(())
                }
            }
        })
        .await;

    assert!(result.is_ok(), "polling loop should stop cleanly");
    assert_eq!(handled_calls.load(Ordering::SeqCst), 1);

    let cursor_snapshot = cursor_store.snapshot();
    assert_eq!(cursor_snapshot.pending, Some(777));
    assert_eq!(cursor_snapshot.committed, Some(777));
    assert_eq!(cursor_snapshot.set_calls, 1);
    assert_eq!(cursor_snapshot.commit_calls, 1);

    let requests = executor.take_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        query_value(&requests[0], "marker"),
        None,
        "first fetch should omit marker when no committed marker exists"
    );
}

#[tokio::test]
async fn polling_loop_does_not_commit_marker_when_handler_fails() {
    let executor = MockHttpExecutor::default();
    executor.queue_updates_page_response(Some(991), vec![make_update_fixture(1_725_435_901_000)]);

    let cursor_store = MockCursorStore::default();
    let polling_client = make_polling_client(executor.clone(), cursor_store.clone());
    let polling_loop = PollingLoop::new(
        polling_client,
        PollingLoopConfig {
            poll_interval: Duration::from_millis(1),
            max_updates_per_tick: 0,
            commit_strategy: CommitStrategy::AfterSuccess,
        },
    );

    let handled_calls = Arc::new(AtomicUsize::new(0));
    let (_stop_tx, stop_rx) = watch::channel(false);

    let result = polling_loop
        .run(stop_rx, {
            let handled_calls = Arc::clone(&handled_calls);
            move |_| {
                let handled_calls = Arc::clone(&handled_calls);
                async move {
                    handled_calls.fetch_add(1, Ordering::SeqCst);
                    Err("handler failure".to_owned())
                }
            }
        })
        .await;

    assert!(matches!(
        result,
        Err(PollingLoopError::Handler(message)) if message == "handler failure"
    ));
    assert_eq!(handled_calls.load(Ordering::SeqCst), 1);

    let cursor_snapshot = cursor_store.snapshot();
    assert_eq!(cursor_snapshot.pending, Some(991));
    assert_eq!(cursor_snapshot.committed, None);
    assert_eq!(cursor_snapshot.set_calls, 1);
    assert_eq!(cursor_snapshot.commit_calls, 0);

    let requests = executor.take_requests();
    assert_eq!(requests.len(), 1);
}
