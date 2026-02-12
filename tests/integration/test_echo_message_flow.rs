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
    api::{client::BotApiClient, messages_send::SendMessageRequest},
    auth::credentials::BotCredentials,
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    config::client_config::ClientConfig,
    errors::api_error::ApiError,
    models::new_message_body::NewMessageBody,
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
async fn send_then_reply_echo_flow_uses_typed_client_calls() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(
        StatusCode::OK,
        json!({
            "message": {
                "message_id": "msg-inbound",
                "recipient": {"chat_id": 777},
                "body": {"text": "hello sdk"}
            }
        }),
    );
    executor.queue_json_response(
        StatusCode::OK,
        json!({
            "message": {
                "message_id": "msg-reply",
                "recipient": {"chat_id": 777},
                "body": {"text": "echo: hello sdk"}
            }
        }),
    );

    let client = make_client(executor.clone());

    let incoming_request =
        SendMessageRequest::to_chat(777, NewMessageBody::new().with_text("hello sdk"));
    let incoming_message = client
        .send_message(&incoming_request)
        .await
        .expect("first send response should decode");

    assert_eq!(incoming_message.id(), Some("msg-inbound"));
    let incoming_text = incoming_message
        .body()
        .and_then(|body| body.text())
        .expect("incoming message text should be present");
    let chat_id = incoming_message
        .recipient()
        .and_then(maxobot_core::models::message::MessageRecipient::chat_id)
        .expect("incoming message chat recipient should be present");

    let reply_text = format!("echo: {incoming_text}");
    let reply_request =
        SendMessageRequest::to_chat(chat_id, NewMessageBody::new().with_text(reply_text.clone()));
    let reply_message = client
        .send_message(&reply_request)
        .await
        .expect("reply send response should decode");

    assert_eq!(reply_message.id(), Some("msg-reply"));
    assert_eq!(
        reply_message
            .body()
            .and_then(|body| body.text())
            .expect("reply message text should be present"),
        reply_text
    );
    assert_eq!(
        reply_message
            .recipient()
            .and_then(maxobot_core::models::message::MessageRecipient::chat_id),
        Some(chat_id)
    );

    let captured_requests = executor.take_requests();
    assert_eq!(
        captured_requests.len(),
        2,
        "exactly two requests are expected"
    );

    let first_request = &captured_requests[0];
    assert_common_contract(first_request, Method::POST, "/messages");
    assert_eq!(query_value(first_request, "chat_id"), Some("777"));
    assert_eq!(
        first_request.body_json,
        Some(json!({
            "text": "hello sdk"
        }))
    );

    let second_request = &captured_requests[1];
    assert_common_contract(second_request, Method::POST, "/messages");
    assert_eq!(query_value(second_request, "chat_id"), Some("777"));
    assert_eq!(
        second_request.body_json,
        Some(json!({
            "text": "echo: hello sdk"
        }))
    );
}
