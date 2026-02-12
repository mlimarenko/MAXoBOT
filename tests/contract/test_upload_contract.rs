use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use bytes::Bytes;
use http::{Method, StatusCode};
use reqwest::header::{AUTHORIZATION, HeaderMap};
use serde_json::json;

use maxobot_core::{
    api::{client::BotApiClient, uploads::UploadType},
    auth::credentials::BotCredentials,
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    config::client_config::ClientConfig,
    errors::api_error::ApiError,
};

const TEST_TOKEN: &str = "upload-contract-token";

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
    fn queue_json_response(&self, body: serde_json::Value) {
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::from(serde_json::to_vec(&body).expect("response should serialize")),
        };

        self.state
            .lock()
            .expect("mock lock should not be poisoned")
            .queued_responses
            .push_back(Ok(response));
    }

    fn take_single_request(&self) -> HttpRequest {
        let mut state = self
            .state
            .lock()
            .expect("mock lock should not be poisoned");
        assert_eq!(state.captured_requests.len(), 1);
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
            .expect("mock lock should not be poisoned");
        state.captured_requests.push(request);
        state
            .queued_responses
            .pop_front()
            .expect("queued response should exist")
    }
}

fn make_client(executor: MockHttpExecutor) -> BotApiClient<MockHttpExecutor> {
    let config = ClientConfig::default();
    let credentials = BotCredentials::new(TEST_TOKEN).expect("test token should be valid");
    BotApiClient::new(executor, config, credentials).expect("client should build")
}

fn assert_upload_request(request: &HttpRequest, expected_type: &str) {
    assert_eq!(request.method, Method::POST);
    assert_eq!(request.url.path(), "/uploads");
    assert_eq!(
        request.query,
        vec![("type".to_owned(), expected_type.to_owned())]
    );
    assert_eq!(
        request
            .headers
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok()),
        Some(TEST_TOKEN)
    );
}

#[tokio::test]
async fn create_upload_ticket_for_image_decodes_url_and_token() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(json!({
        "url": "https://upload.max.ru/image-ticket",
        "token": "image-token"
    }));
    let client = make_client(executor.clone());

    let ticket = client
        .create_upload_ticket(UploadType::Image)
        .await
        .expect("ticket should decode");

    assert_eq!(ticket.url(), Some("https://upload.max.ru/image-ticket"));
    assert_eq!(ticket.token(), Some("image-token"));
    assert!(ticket.has_token());
    assert_upload_request(&executor.take_single_request(), "image");
}

#[tokio::test]
async fn create_upload_ticket_for_video_decodes_token() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(json!({
        "url": "https://upload.max.ru/video-ticket",
        "token": "video-token"
    }));
    let client = make_client(executor.clone());

    let ticket = client
        .create_upload_ticket(UploadType::Video)
        .await
        .expect("ticket should decode");

    assert_eq!(ticket.token(), Some("video-token"));
    assert_upload_request(&executor.take_single_request(), "video");
}

#[tokio::test]
async fn create_upload_ticket_for_audio_handles_null_token() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(json!({
        "url": "https://upload.max.ru/audio-ticket",
        "token": null
    }));
    let client = make_client(executor.clone());

    let ticket = client
        .create_upload_ticket(UploadType::Audio)
        .await
        .expect("ticket should decode");

    assert_eq!(ticket.token(), None);
    assert_upload_request(&executor.take_single_request(), "audio");
}

#[tokio::test]
async fn create_upload_ticket_for_file_handles_missing_token() {
    let executor = MockHttpExecutor::default();
    executor.queue_json_response(json!({
        "url": "https://upload.max.ru/file-ticket"
    }));
    let client = make_client(executor.clone());

    let ticket = client
        .create_upload_ticket(UploadType::File)
        .await
        .expect("ticket should decode");

    assert_eq!(ticket.token(), None);
    assert_upload_request(&executor.take_single_request(), "file");
}
