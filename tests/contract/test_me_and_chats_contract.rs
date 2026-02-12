use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use bytes::Bytes;
use http::{Method, StatusCode};
use maxobot_core::{
    api::client::BotApiClient,
    auth::credentials::BotCredentials,
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    config::client_config::ClientConfig,
    errors::api_error::ApiError,
};
use reqwest::header::{AUTHORIZATION, HeaderMap};

/// Lightweight executor mock for request/response contract assertions.
#[derive(Debug, Clone)]
struct MockHttpExecutor {
    recorded_requests: Arc<Mutex<Vec<HttpRequest>>>,
    canned_responses: Arc<Mutex<VecDeque<HttpResponse>>>,
}

impl MockHttpExecutor {
    fn with_json_bodies(bodies: &[&str]) -> Self {
        let responses = bodies
            .iter()
            .map(|body| HttpResponse {
                status: StatusCode::OK,
                headers: HeaderMap::new(),
                body: Bytes::from((*body).to_owned()),
            })
            .collect();

        Self {
            recorded_requests: Arc::new(Mutex::new(Vec::new())),
            canned_responses: Arc::new(Mutex::new(responses)),
        }
    }

    fn take_requests(&self) -> Vec<HttpRequest> {
        let mut guard = self
            .recorded_requests
            .lock()
            .expect("mock recorded request mutex should not be poisoned");
        std::mem::take(&mut *guard)
    }
}

#[async_trait]
impl HttpExecutor for MockHttpExecutor {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ApiError> {
        self.recorded_requests
            .lock()
            .expect("mock recorded request mutex should not be poisoned")
            .push(request);

        self.canned_responses
            .lock()
            .expect("mock canned response mutex should not be poisoned")
            .pop_front()
            .ok_or_else(|| {
                ApiError::InvalidConfiguration(
                    "mock executor ran out of canned responses".to_owned(),
                )
            })
    }
}

fn build_client(executor: MockHttpExecutor) -> BotApiClient<MockHttpExecutor> {
    let config = ClientConfig::default();
    let credentials = BotCredentials::new("contract-token").expect("test token should be valid");
    BotApiClient::new(executor, config, credentials).expect("client config should be valid")
}

fn assert_get_request_contract(request: &HttpRequest, expected_path: &str) {
    assert_eq!(request.method, Method::GET);
    assert_eq!(request.url.path(), expected_path);
    assert!(request.query.is_empty());
    assert_eq!(request.url.query(), None);
    assert_eq!(
        request
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
        Some("contract-token")
    );
}

#[tokio::test]
async fn get_me_contract_validates_request_and_decodes_user() {
    let executor =
        MockHttpExecutor::with_json_bodies(&[r#"{"user_id":7,"name":"MAX Bot","is_bot":true}"#]);
    let client = build_client(executor.clone());

    let user = client.get_me().await.expect("get_me should decode user");
    let requests = executor.take_requests();

    assert_eq!(requests.len(), 1);
    assert_get_request_contract(&requests[0], "/me");
    assert_eq!(user.id(), Some(7));
    assert_eq!(user.name(), Some("MAX Bot"));
    assert!(user.is_bot());
}

#[tokio::test]
async fn get_chats_contract_validates_request_and_decodes_chat_list() {
    let executor = MockHttpExecutor::with_json_bodies(&[
        r#"{"chats":[{"chat_id":11,"type":"group","title":"Core Team"},{"chat_id":12,"type":"private","username":"alice"}]}"#,
    ]);
    let client = build_client(executor.clone());

    let chats = client
        .get_chats()
        .await
        .expect("get_chats should decode chats");
    let requests = executor.take_requests();

    assert_eq!(requests.len(), 1);
    assert_get_request_contract(&requests[0], "/chats");
    assert_eq!(chats.len(), 2);
    assert_eq!(chats[0].id(), Some(11));
    assert_eq!(chats[0].title(), Some("Core Team"));
    assert_eq!(chats[1].id(), Some(12));
    assert_eq!(chats[1].username(), Some("alice"));
}

#[tokio::test]
async fn get_chat_by_id_contract_validates_request_and_decodes_chat() {
    let executor =
        MockHttpExecutor::with_json_bodies(&[r#"{"chat_id":42,"type":"group","title":"Backend"}"#]);
    let client = build_client(executor.clone());

    let chat = client
        .get_chat_by_id(42)
        .await
        .expect("get_chat_by_id should decode chat");
    let requests = executor.take_requests();

    assert_eq!(requests.len(), 1);
    assert_get_request_contract(&requests[0], "/chats/42");
    assert_eq!(chat.id(), Some(42));
    assert_eq!(chat.chat_type(), Some("group"));
    assert_eq!(chat.title(), Some("Backend"));
}
