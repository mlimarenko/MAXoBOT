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
    api::{callback_answers::CallbackAnswerRequest, client::BotApiClient},
    auth::credentials::BotCredentials,
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    config::client_config::ClientConfig,
    errors::api_error::ApiError,
    models::{action_result::ActionResult, new_message_body::NewMessageBody},
};

const TEST_TOKEN: &str = "callback-contract-token";

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

fn query_value<'a>(request: &'a HttpRequest, key: &str) -> Option<&'a str> {
    request
        .query
        .iter()
        .find_map(|(query_key, query_value)| (query_key == key).then_some(query_value.as_str()))
}

fn assert_callback_answer_contract(request: &HttpRequest) {
    assert_eq!(request.method, Method::POST);
    assert_eq!(request.url.path(), "/answers");
    assert_eq!(request.url.query(), None);

    let authorization = request
        .headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok());
    assert_eq!(authorization, Some(TEST_TOKEN));
}

#[tokio::test]
async fn answer_callback_serializes_query_message_notification_and_decodes_action_result() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({"success": true, "message": "answered"}),
    );

    let message = NewMessageBody::new()
        .with_text("processing")
        .with_notify(false);
    let request = CallbackAnswerRequest::new()
        .with_message(message)
        .with_notification("done");
    let expected_body = serde_json::to_value(&request).expect("request should serialize");

    let client = make_client(executor.clone());
    let result: ActionResult = client
        .answer_callback("cb-123", &request)
        .await
        .expect("callback answer should decode");

    assert!(result.is_success());
    assert_eq!(result.message(), Some("answered"));

    let captured_request = executor.take_single_request();
    assert_callback_answer_contract(&captured_request);
    assert_eq!(
        captured_request.query,
        vec![("callback_id".to_owned(), "cb-123".to_owned())]
    );
    assert_eq!(captured_request.body_json, Some(expected_body));
}

#[tokio::test]
async fn answer_callback_notification_only_omits_message_field() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(StatusCode::OK, json!({"success": true}));

    let request = CallbackAnswerRequest::new().with_notification("visible notice");

    let client = make_client(executor.clone());
    let result: ActionResult = client
        .answer_callback("cb-42", &request)
        .await
        .expect("notification-only callback should decode");

    assert!(result.is_success());
    assert_eq!(result.message(), None);

    let captured_request = executor.take_single_request();
    assert_callback_answer_contract(&captured_request);
    assert_eq!(query_value(&captured_request, "callback_id"), Some("cb-42"));
    assert_eq!(
        captured_request.body_json,
        Some(json!({"notification": "visible notice"}))
    );
}

#[tokio::test]
async fn answer_callback_message_only_omits_notification_field() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(StatusCode::OK, json!({"success": true}));

    let message = NewMessageBody::new().with_text("ack");
    let request = CallbackAnswerRequest::new().with_message(message);
    let expected_body = serde_json::to_value(&request).expect("request should serialize");

    let client = make_client(executor.clone());
    let result: ActionResult = client
        .answer_callback("cb-99", &request)
        .await
        .expect("message-only callback should decode");

    assert!(result.is_success());
    assert_eq!(result.message(), None);

    let captured_request = executor.take_single_request();
    assert_callback_answer_contract(&captured_request);
    assert_eq!(query_value(&captured_request, "callback_id"), Some("cb-99"));
    assert_eq!(captured_request.body_json, Some(expected_body));
}
