//! Request builder for normalized HTTP requests.

use http::Method;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Serialize;
use serde_json::Value;
use url::Url;

use crate::auth::authorization::ensure_no_query_auth;
use crate::client::http_executor::HttpRequest;
use crate::errors::api_error::ApiError;

const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// Builder for [`HttpRequest`] instances.
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    base_url: Url,
    method: Method,
    path: String,
    query: Vec<(String, String)>,
    headers: HeaderMap,
    body_json: Option<Value>,
    idempotency_key: Option<String>,
}

impl RequestBuilder {
    /// Creates a new request builder.
    #[must_use]
    pub fn new(base_url: Url, method: Method, path: impl Into<String>) -> Self {
        Self {
            base_url,
            method,
            path: path.into(),
            query: Vec::new(),
            headers: HeaderMap::new(),
            body_json: None,
            idempotency_key: None,
        }
    }

    /// Adds a query parameter.
    #[must_use]
    pub fn with_query_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    /// Adds a header.
    pub fn with_header(
        mut self,
        name: HeaderName,
        value: impl AsRef<str>,
    ) -> Result<Self, ApiError> {
        let value =
            HeaderValue::from_str(value.as_ref()).map_err(|error| ApiError::InvalidHeader {
                header: name.to_string(),
                reason: error.to_string(),
            })?;
        self.headers.insert(name, value);
        Ok(self)
    }

    /// Sets an idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, value: impl Into<String>) -> Self {
        self.idempotency_key = Some(value.into());
        self
    }

    /// Adds a JSON body.
    pub fn with_body<T: Serialize>(mut self, body: &T) -> Result<Self, ApiError> {
        self.body_json = Some(
            serde_json::to_value(body)
                .map_err(|error| ApiError::InvalidResponseShape(error.to_string()))?,
        );
        Ok(self)
    }

    /// Builds a finalized request.
    pub fn build(mut self) -> Result<HttpRequest, ApiError> {
        ensure_no_query_auth(&self.query)?;

        if let Some(key) = self.idempotency_key.take() {
            let value = HeaderValue::from_str(&key).map_err(|error| ApiError::InvalidHeader {
                header: IDEMPOTENCY_KEY_HEADER.to_owned(),
                reason: error.to_string(),
            })?;

            self.headers
                .insert(HeaderName::from_static("idempotency-key"), value);
        }

        let path = normalize_path(&self.path);
        let url = self
            .base_url
            .join(&path)
            .map_err(|_| ApiError::UrlJoinError {
                base: self.base_url.to_string(),
                path,
            })?;

        Ok(HttpRequest {
            method: self.method,
            url,
            headers: self.headers,
            query: self.query,
            body_json: self.body_json,
        })
    }
}

fn normalize_path(path: &str) -> String {
    if path.starts_with('/') {
        path.trim_start_matches('/').to_owned()
    } else {
        path.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use http::Method;
    use reqwest::header::AUTHORIZATION;
    use serde_json::json;
    use url::Url;

    use super::RequestBuilder;

    #[test]
    fn builds_request_with_query_and_body() {
        let base = Url::parse("https://platform-api.max.ru/").expect("valid URL");
        let request = RequestBuilder::new(base, Method::POST, "/messages")
            .with_query_param("chat_id", "1")
            .with_header(AUTHORIZATION, "token")
            .expect("header")
            .with_body(&json!({"text":"hello"}))
            .expect("body")
            .build()
            .expect("request");

        assert_eq!(request.url.as_str(), "https://platform-api.max.ru/messages");
        assert_eq!(request.query.len(), 1);
        assert_eq!(request.body_json, Some(json!({"text":"hello"})));
    }
}
