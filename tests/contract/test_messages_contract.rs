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
    api::{
        client::BotApiClient, messages_edit::EditMessageRequest, messages_send::SendMessageRequest,
    },
    auth::credentials::BotCredentials,
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    config::client_config::ClientConfig,
    errors::api_error::ApiError,
    models::{action_result::ActionResult, new_message_body::NewMessageBody},
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

fn assert_common_contract(request: &HttpRequest, method: Method, path: &str) {
    assert_eq!(request.method, method);
    assert_eq!(request.url.path(), path);

    let authorization = request
        .headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok());
    assert_eq!(authorization, Some(TEST_TOKEN));
}

#[tokio::test]
async fn get_messages_uses_get_messages_path_and_decodes_payload() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({
            "messages": [
                {"message_id": "msg-1", "body": {"text": "first"}},
                {"message_id": "msg-2", "body": {"text": "second"}}
            ]
        }),
    );

    let client = make_client(executor.clone());
    let messages = client
        .get_messages()
        .await
        .expect("messages response should decode");

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].id(), Some("msg-1"));
    assert_eq!(messages[1].id(), Some("msg-2"));

    let request = executor.take_single_request();
    assert_common_contract(&request, Method::GET, "/messages");
    assert!(request.query.is_empty());
    assert!(request.body_json.is_none());
}

#[tokio::test]
async fn get_message_by_id_uses_path_parameter_and_decodes_message() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({"message": {"message_id": "msg-42", "body": {"text": "hello"}}}),
    );

    let client = make_client(executor.clone());
    let message = client
        .get_message_by_id("msg-42")
        .await
        .expect("message response should decode");

    assert_eq!(message.id(), Some("msg-42"));

    let request = executor.take_single_request();
    assert_common_contract(&request, Method::GET, "/messages/msg-42");
    assert!(request.query.is_empty());
    assert!(request.body_json.is_none());
}

#[tokio::test]
async fn send_message_serializes_body_and_query_parameters() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({"message": {"message_id": "msg-77", "body": {"text": "hello"}}}),
    );

    let body = NewMessageBody::new().with_text("hello");
    let request_body = body.clone();
    let request = SendMessageRequest::to_chat(777, body).with_disable_link_preview(true);

    let client = make_client(executor.clone());
    let message = client
        .send_message(&request)
        .await
        .expect("send response should decode");

    assert_eq!(message.id(), Some("msg-77"));

    let captured_request = executor.take_single_request();
    assert_common_contract(&captured_request, Method::POST, "/messages");
    assert_eq!(query_value(&captured_request, "chat_id"), Some("777"));
    assert_eq!(
        query_value(&captured_request, "disable_link_preview"),
        Some("true")
    );
    assert!(query_value(&captured_request, "user_id").is_none());

    let expected_body = serde_json::to_value(&request_body).expect("request body should serialize");
    assert_eq!(captured_request.body_json, Some(expected_body));
}

#[tokio::test]
async fn edit_message_serializes_message_id_query_and_body() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({"message": {"message_id": "msg-91", "body": {"text": "edited"}}}),
    );

    let body = NewMessageBody::new().with_text("edited");
    let expected_body = serde_json::to_value(&body).expect("request body should serialize");
    let request = EditMessageRequest::new("msg-91", body);

    let client = make_client(executor.clone());
    let message = client
        .edit_message(&request)
        .await
        .expect("edit response should decode");

    assert_eq!(message.id(), Some("msg-91"));

    let captured_request = executor.take_single_request();
    assert_common_contract(&captured_request, Method::PUT, "/messages");
    assert_eq!(query_value(&captured_request, "message_id"), Some("msg-91"));
    assert_eq!(captured_request.body_json, Some(expected_body));
}

#[tokio::test]
async fn delete_message_uses_delete_messages_query_and_decodes_action_result() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({"success": true, "message": "deleted"}),
    );

    let client = make_client(executor.clone());
    let result: ActionResult = client
        .delete_message("msg-13")
        .await
        .expect("delete response should decode");

    assert!(result.is_success());
    assert_eq!(result.message(), Some("deleted"));

    let captured_request = executor.take_single_request();
    assert_common_contract(&captured_request, Method::DELETE, "/messages");
    assert_eq!(query_value(&captured_request, "message_id"), Some("msg-13"));
    assert!(captured_request.body_json.is_none());
}
