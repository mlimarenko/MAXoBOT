//! Multipart uploader for binary media transfer via ticket URLs.

use bytes::Bytes;
use http::StatusCode;
use mime::Mime;
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;
use uuid::Uuid;

use crate::errors::api_error::{ApiError, redact_sensitive};

const DEFAULT_FIELD_NAME: &str = "file";
const DEFAULT_FILE_NAME: &str = "upload.bin";
const DEFAULT_CONTENT_TYPE: &str = "application/octet-stream";

/// Binary payload configuration for multipart upload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartUploadRequest {
    upload_url: String,
    binary: Vec<u8>,
    field_name: Option<String>,
    file_name: Option<String>,
    content_type: Option<String>,
}

impl MultipartUploadRequest {
    /// Creates a multipart upload request.
    #[must_use]
    pub fn new(upload_url: impl Into<String>, binary: impl Into<Vec<u8>>) -> Self {
        Self {
            upload_url: upload_url.into(),
            binary: binary.into(),
            field_name: None,
            file_name: None,
            content_type: None,
        }
    }

    /// Overrides multipart form field name used for the binary payload.
    #[must_use]
    pub fn with_field_name(mut self, field_name: impl Into<String>) -> Self {
        self.field_name = Some(field_name.into());
        self
    }

    /// Overrides file name sent in the multipart `Content-Disposition` header.
    #[must_use]
    pub fn with_file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    /// Overrides binary part content type.
    #[must_use]
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Returns target upload URL string as provided by caller.
    #[must_use]
    pub fn upload_url(&self) -> &str {
        &self.upload_url
    }

    /// Returns raw binary payload bytes.
    #[must_use]
    pub fn binary(&self) -> &[u8] {
        &self.binary
    }

    fn prepare(self) -> Result<PreparedMultipartUpload, ApiError> {
        if self.binary.is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "multipart upload payload must not be empty".to_owned(),
            ));
        }

        let upload_url = parse_upload_url(&self.upload_url)?;
        let field_name =
            normalize_required(self.field_name, "multipart field name", DEFAULT_FIELD_NAME)?;
        let file_name =
            normalize_required(self.file_name, "multipart file name", DEFAULT_FILE_NAME)?;
        validate_header_fragment(&field_name, "multipart field name")?;
        validate_header_fragment(&file_name, "multipart file name")?;
        let content_type = parse_content_type(self.content_type)?;

        Ok(PreparedMultipartUpload {
            upload_url,
            binary: self.binary,
            field_name,
            file_name,
            content_type,
        })
    }
}

/// Captured response from binary upload endpoint.
#[derive(Debug, Clone)]
pub struct MultipartUploadResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl MultipartUploadResponse {
    fn new(status: StatusCode, headers: HeaderMap, body: Bytes) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    /// Returns HTTP status from upload endpoint.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns response headers from upload endpoint.
    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns raw response body bytes.
    #[must_use]
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Deserializes response body as JSON payload.
    pub fn json<T>(&self) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice::<T>(&self.body).map_err(|source| ApiError::ResponseDecode {
            source,
            body_preview: body_preview(&self.body),
        })
    }
}

/// Reqwest-backed multipart uploader for media upload URLs.
#[derive(Debug, Clone)]
pub struct MultipartUploader {
    client: reqwest::Client,
}

impl MultipartUploader {
    /// Creates uploader backed by provided reqwest client.
    #[must_use]
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Builds a POST multipart request without executing network I/O.
    pub fn build_request(
        &self,
        request: MultipartUploadRequest,
    ) -> Result<reqwest::Request, ApiError> {
        let prepared = request.prepare()?;
        let multipart = build_multipart_body(&prepared)?;
        self.client
            .post(prepared.upload_url)
            .header(CONTENT_TYPE, multipart.content_type_header)
            .body(multipart.body)
            .build()
            .map_err(ApiError::from)
    }

    /// Uploads binary payload to provided ticket URL using multipart/form-data.
    pub async fn upload_by_url(
        &self,
        request: MultipartUploadRequest,
    ) -> Result<MultipartUploadResponse, ApiError> {
        let request = self.build_request(request)?;
        let response = self.client.execute(request).await.map_err(ApiError::from)?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await.map_err(ApiError::from)?;

        if !status.is_success() {
            let (code, message) = parse_error_payload(&body);
            return Err(ApiError::from_status(status, code, message));
        }

        Ok(MultipartUploadResponse::new(status, headers, body))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedMultipartUpload {
    upload_url: Url,
    binary: Vec<u8>,
    field_name: String,
    file_name: String,
    content_type: Mime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BuiltMultipartBody {
    content_type_header: String,
    body: Vec<u8>,
}

fn normalize_required(
    value: Option<String>,
    field_name: &str,
    default_value: &str,
) -> Result<String, ApiError> {
    let value = value.unwrap_or_else(|| default_value.to_owned());

    if value.trim().is_empty() {
        return Err(ApiError::InvalidConfiguration(format!(
            "{field_name} must not be empty"
        )));
    }

    Ok(value)
}

fn parse_upload_url(upload_url: &str) -> Result<Url, ApiError> {
    if upload_url.trim().is_empty() {
        return Err(ApiError::InvalidConfiguration(
            "upload URL must not be empty".to_owned(),
        ));
    }

    let parsed = Url::parse(upload_url)
        .map_err(|error| ApiError::InvalidConfiguration(format!("invalid upload URL: {error}")))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ApiError::InvalidConfiguration(format!(
            "upload URL scheme must be http or https, got `{}`",
            parsed.scheme()
        )));
    }

    Ok(parsed)
}

fn parse_content_type(content_type: Option<String>) -> Result<Mime, ApiError> {
    let value = content_type.unwrap_or_else(|| DEFAULT_CONTENT_TYPE.to_owned());

    if value.trim().is_empty() {
        return Err(ApiError::InvalidConfiguration(
            "multipart content type must not be empty".to_owned(),
        ));
    }

    value.trim().parse::<Mime>().map_err(|error| {
        ApiError::InvalidConfiguration(format!("invalid multipart content type `{value}`: {error}"))
    })
}

fn build_multipart_body(
    prepared: &PreparedMultipartUpload,
) -> Result<BuiltMultipartBody, ApiError> {
    validate_header_fragment(&prepared.field_name, "multipart field name")?;
    validate_header_fragment(&prepared.file_name, "multipart file name")?;

    let boundary = format!("maxobot-{}", Uuid::now_v7().simple());
    let mut body = Vec::with_capacity(prepared.binary.len().saturating_add(512));

    append_multipart_line(&mut body, &format!("--{boundary}"));
    append_multipart_line(
        &mut body,
        &format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"",
            escape_quoted(&prepared.field_name),
            escape_quoted(&prepared.file_name)
        ),
    );
    append_multipart_line(
        &mut body,
        &format!("Content-Type: {}", prepared.content_type),
    );
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(&prepared.binary);
    body.extend_from_slice(b"\r\n");
    append_multipart_line(&mut body, &format!("--{boundary}--"));

    Ok(BuiltMultipartBody {
        content_type_header: format!("multipart/form-data; boundary={boundary}"),
        body,
    })
}

fn append_multipart_line(output: &mut Vec<u8>, line: &str) {
    output.extend_from_slice(line.as_bytes());
    output.extend_from_slice(b"\r\n");
}

fn validate_header_fragment(value: &str, field_name: &str) -> Result<(), ApiError> {
    if value.contains(['\r', '\n']) {
        return Err(ApiError::InvalidConfiguration(format!(
            "{field_name} must not contain CR/LF characters"
        )));
    }

    Ok(())
}

fn escape_quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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

fn body_preview(body: &Bytes) -> String {
    let utf8 = String::from_utf8_lossy(body);
    let preview: String = utf8.chars().take(256).collect();
    redact_sensitive(&preview)
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http::Method;
    use reqwest::header::CONTENT_TYPE;

    use super::{
        MultipartUploadRequest, MultipartUploadResponse, MultipartUploader, parse_error_payload,
    };
    use crate::errors::api_error::ApiError;

    #[test]
    fn prepare_uses_defaults_for_field_file_name_and_content_type() {
        let prepared =
            MultipartUploadRequest::new("https://upload.max.ru/ticket", vec![1_u8, 2_u8, 3_u8])
                .prepare()
                .expect("request should prepare");

        assert_eq!(prepared.upload_url.as_str(), "https://upload.max.ru/ticket");
        assert_eq!(prepared.field_name, "file");
        assert_eq!(prepared.file_name, "upload.bin");
        assert_eq!(prepared.content_type.as_ref(), "application/octet-stream");
    }

    #[test]
    fn prepare_applies_explicit_content_type() {
        let prepared = MultipartUploadRequest::new("https://upload.max.ru/ticket", vec![9_u8])
            .with_content_type("image/png")
            .prepare()
            .expect("request should prepare");

        assert_eq!(prepared.content_type.as_ref(), "image/png");
    }

    #[test]
    fn prepare_rejects_empty_binary_payload() {
        let error = MultipartUploadRequest::new("https://upload.max.ru/ticket", Vec::<u8>::new())
            .prepare()
            .expect_err("empty payload must fail");

        assert!(matches!(error, ApiError::InvalidConfiguration(_)));
    }

    #[test]
    fn prepare_rejects_non_http_url_scheme() {
        let error = MultipartUploadRequest::new("ftp://upload.max.ru/ticket", vec![1_u8])
            .prepare()
            .expect_err("invalid scheme must fail");

        assert!(matches!(error, ApiError::InvalidConfiguration(_)));
    }

    #[test]
    fn prepare_rejects_invalid_content_type() {
        let error = MultipartUploadRequest::new("https://upload.max.ru/ticket", vec![1_u8])
            .with_content_type("invalid mime")
            .prepare()
            .expect_err("invalid content type must fail");

        assert!(matches!(error, ApiError::InvalidConfiguration(_)));
    }

    #[test]
    fn build_request_creates_post_multipart_request() {
        let uploader = MultipartUploader::new(reqwest::Client::new());
        let request = uploader
            .build_request(
                MultipartUploadRequest::new("https://upload.max.ru/ticket", vec![1_u8])
                    .with_field_name("media")
                    .with_file_name("photo.png")
                    .with_content_type("image/png"),
            )
            .expect("request should build");

        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.url().as_str(), "https://upload.max.ru/ticket");
        let content_type = request
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|header| header.to_str().ok())
            .expect("multipart content-type header should exist");
        assert!(content_type.starts_with("multipart/form-data; boundary="));

        let body = request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .expect("multipart body should be in-memory");
        let body_text = String::from_utf8_lossy(body);
        assert!(
            body_text
                .contains("Content-Disposition: form-data; name=\"media\"; filename=\"photo.png\"")
        );
        assert!(body_text.contains("Content-Type: image/png"));
    }

    #[test]
    fn prepare_rejects_field_name_with_crlf() {
        let error = MultipartUploadRequest::new("https://upload.max.ru/ticket", vec![1_u8])
            .with_field_name("media\r\nx")
            .prepare()
            .expect_err("invalid field name must fail");

        assert!(matches!(error, ApiError::InvalidConfiguration(_)));
    }

    #[test]
    fn response_json_maps_decode_errors_to_api_error() {
        let response = MultipartUploadResponse::new(
            http::StatusCode::OK,
            reqwest::header::HeaderMap::new(),
            Bytes::from_static(br"{not-json"),
        );

        let error = response
            .json::<serde_json::Value>()
            .expect_err("invalid json must fail");

        assert!(matches!(error, ApiError::ResponseDecode { .. }));
    }

    #[test]
    fn parse_error_payload_extracts_code_and_message() {
        let body = Bytes::from_static(br#"{"code":"attachment.not.ready","message":"try later"}"#);
        let (code, message) = parse_error_payload(&body);

        assert_eq!(code.as_deref(), Some("attachment.not.ready"));
        assert_eq!(message.as_deref(), Some("try later"));
    }
}
