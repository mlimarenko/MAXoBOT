use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use bytes::Bytes;
use http::{Method, StatusCode};
use reqwest::header::{AUTHORIZATION, HeaderMap};
use serde_json::{Value, json};

use maxobot_core::{
    api::{client::BotApiClient, updates::GetUpdatesRequest},
    auth::credentials::BotCredentials,
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    config::client_config::ClientConfig,
    errors::api_error::ApiError,
    updates::update_envelope::{KnownUpdateType, UpdateSource, UpdateType},
};

const TEST_TOKEN: &str = "updates-contract-token";

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

    fn take_single_request(&self) -> HttpRequest {
        let mut state = self
            .state
            .lock()
            .expect("mock executor lock should not be poisoned");

        assert_eq!(
            state.captured_requests.len(),
            1,
            "exactly one request is expected"
        );

        state
            .captured_requests
            .pop()
            .expect("captured request should exist")
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

fn make_client(executor: MockHttpExecutor) -> BotApiClient<MockHttpExecutor> {
    let config = ClientConfig::default();
    let credentials = BotCredentials::new(TEST_TOKEN).expect("test token should be valid");

    BotApiClient::new(executor, config, credentials).expect("client should build")
}

fn assert_updates_contract(request: &HttpRequest) {
    assert_eq!(request.method, Method::GET);
    assert_eq!(request.url.path(), "/updates");
    assert_eq!(request.url.query(), None);

    let authorization = request
        .headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok());
    assert_eq!(authorization, Some(TEST_TOKEN));
}

#[tokio::test]
async fn get_updates_serializes_query_and_decodes_updates_page() {
    let first_update_raw = json!({
        "update_type": "message_created",
        "timestamp": 1_700_000_000_123_i64,
        "payload": {"message_id": "msg-1", "text": "hello"}
    });
    let second_update_raw = json!({
        "update_type": "future_update_kind",
        "timestamp": 1_700_000_000_456_i64,
        "meta": {"chat_id": 77}
    });

    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({
            "updates": [first_update_raw.clone(), second_update_raw.clone()],
            "marker": 9_001_i64
        }),
    );

    let request = GetUpdatesRequest {
        limit: Some(25),
        timeout: Some(45),
        marker: Some(123_456_789_i64),
        types: vec!["message_created".to_owned(), "message_callback".to_owned()],
    };

    let client = make_client(executor.clone());
    let page = client
        .get_updates(&request)
        .await
        .expect("updates response should decode");

    assert_eq!(page.marker, Some(9_001_i64));
    assert_eq!(page.updates.len(), 2);

    let first = &page.updates[0];
    assert!(matches!(
        &first.update_type,
        UpdateType::Known(KnownUpdateType::MessageCreated)
    ));
    assert_eq!(first.timestamp, 1_700_000_000_123_i64);
    assert_eq!(first.source, UpdateSource::Polling);
    assert_eq!(
        first.payload,
        json!({"payload": {"message_id": "msg-1", "text": "hello"}})
    );
    assert_eq!(first.raw, first_update_raw);

    let second = &page.updates[1];
    assert!(matches!(
        &second.update_type,
        UpdateType::Unknown(value) if value == "future_update_kind"
    ));
    assert_eq!(second.timestamp, 1_700_000_000_456_i64);
    assert_eq!(second.source, UpdateSource::Polling);
    assert_eq!(second.payload, json!({"meta": {"chat_id": 77}}));
    assert_eq!(second.raw, second_update_raw);

    let captured_request = executor.take_single_request();
    assert_updates_contract(&captured_request);
    assert_eq!(
        captured_request.query,
        vec![
            ("limit".to_owned(), "25".to_owned()),
            ("timeout".to_owned(), "45".to_owned()),
            ("marker".to_owned(), "123456789".to_owned()),
            (
                "types".to_owned(),
                "message_created,message_callback".to_owned()
            ),
        ]
    );
    assert!(captured_request.body_json.is_none());
}
