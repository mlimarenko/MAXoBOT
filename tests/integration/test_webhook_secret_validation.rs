use http::{HeaderMap, HeaderValue};
use maxobot_webhook::verifier::{DEFAULT_SECRET_HEADER, WebhookVerifier, WebhookVerifyError};

#[test]
fn webhook_secret_valid_header_passes() {
    let verifier = WebhookVerifier::new(Some("secret-value".to_owned()));
    let mut headers = HeaderMap::new();
    headers.insert(
        DEFAULT_SECRET_HEADER,
        HeaderValue::from_static("secret-value"),
    );

    assert_eq!(verifier.verify(&headers), Ok(()));
}

#[test]
fn webhook_secret_invalid_header_fails() {
    let verifier = WebhookVerifier::new(Some("secret-value".to_owned()));
    let mut headers = HeaderMap::new();
    headers.insert(
        DEFAULT_SECRET_HEADER,
        HeaderValue::from_static("another-value"),
    );

    assert_eq!(
        verifier.verify(&headers),
        Err(WebhookVerifyError::InvalidSecret)
    );
}

#[test]
fn webhook_secret_missing_header_fails() {
    let verifier = WebhookVerifier::new(Some("secret-value".to_owned()));
    let headers = HeaderMap::new();

    assert!(matches!(
        verifier.verify(&headers),
        Err(WebhookVerifyError::MissingSecretHeader { .. })
    ));
}
