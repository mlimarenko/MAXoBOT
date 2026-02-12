//! HTTP execution abstractions.

use async_trait::async_trait;
use bytes::Bytes;
use http::Method;
use reqwest::header::HeaderMap;
use serde_json::Value;
use url::Url;

use crate::errors::api_error::{ApiError, redact_sensitive};

/// Generic HTTP request representation.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method.
    pub method: Method,
    /// Fully-qualified request URL.
    pub url: Url,
    /// HTTP headers.
    pub headers: HeaderMap,
    /// Query parameters.
    pub query: Vec<(String, String)>,
    /// Optional JSON body.
    pub body_json: Option<Value>,
}

/// Generic HTTP response representation.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// Response status.
    pub status: http::StatusCode,
    /// Response headers.
    pub headers: HeaderMap,
    /// Raw response body bytes.
    pub body: Bytes,
}

/// Executes HTTP requests.
#[async_trait]
pub trait HttpExecutor: Send + Sync {
    /// Executes a request and returns a normalized response.
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ApiError>;
}

/// Reqwest-backed executor implementation.
#[derive(Debug, Clone)]
pub struct ReqwestHttpExecutor {
    client: reqwest::Client,
}

impl ReqwestHttpExecutor {
    /// Creates a new reqwest-backed executor.
    #[must_use]
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl HttpExecutor for ReqwestHttpExecutor {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ApiError> {
        let mut reqwest_request = self.client.request(request.method, request.url);
        reqwest_request = reqwest_request.headers(request.headers);

        if !request.query.is_empty() {
            reqwest_request = reqwest_request.query(&request.query);
        }

        if let Some(body) = request.body_json {
            reqwest_request = reqwest_request.json(&body);
        }

        let response = reqwest_request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await?;

        if !status.is_success() {
            let (code, message) = parse_error_payload(&body);
            return Err(ApiError::from_status(status, code, message));
        }

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn parse_error_payload(body: &Bytes) -> (Option<String>, Option<String>) {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            let object = value.as_object()?;
            let code = object
                .get("code")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let message = object
                .get("message")
                .and_then(Value::as_str)
                .map(redact_sensitive);
            Some((code, message))
        })
        .unwrap_or((None, None))
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::parse_error_payload;

    #[test]
    fn parses_code_and_message_from_error_payload() {
        let payload = Bytes::from_static(br#"{"code":"rate.limited","message":"Too many"}"#);
        let (code, message) = parse_error_payload(&payload);

        assert_eq!(code.as_deref(), Some("rate.limited"));
        assert_eq!(message.as_deref(), Some("Too many"));
    }
}
