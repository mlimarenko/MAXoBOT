use std::time::Duration;

use http::StatusCode;
use reqwest::header::{AUTHORIZATION, HeaderMap};
use serde_json::json;

use maxobot_core::{
    auth::authorization::{ensure_no_query_auth, inject_authorization_header},
    config::client_config::{ClientConfig, ClientConfigValidationError},
    errors::api_error::{ApiError, RetryClass},
    updates::{
        parser::parse_update,
        update_envelope::{UpdateSource, UpdateType},
    },
};

#[test]
fn auth_header_enforcement_uses_header_and_blocks_query_token() {
    let mut headers = HeaderMap::new();
    inject_authorization_header(&mut headers, "bot-token").expect("header should be injected");

    assert_eq!(
        headers
            .get(AUTHORIZATION)
            .expect("authorization header should exist"),
        "bot-token"
    );
    assert!(
        ensure_no_query_auth(&[
            ("chat_id".to_owned(), "1".to_owned()),
            ("access_token".to_owned(), "token".to_owned()),
        ])
        .is_err()
    );
}

#[test]
fn api_error_retry_mapping_covers_rate_limit_server_and_client_errors() {
    let rate_limited = ApiError::from_status(
        StatusCode::TOO_MANY_REQUESTS,
        Some("rate.limited".to_owned()),
        Some("retry".to_owned()),
    );
    let server_error = ApiError::from_status(
        StatusCode::SERVICE_UNAVAILABLE,
        Some("temporary".to_owned()),
        None,
    );
    let client_error = ApiError::from_status(
        StatusCode::BAD_REQUEST,
        Some("validation.failed".to_owned()),
        None,
    );

    assert_eq!(rate_limited.retry_class(), RetryClass::RateLimited);
    assert_eq!(server_error.retry_class(), RetryClass::Backoff);
    assert_eq!(client_error.retry_class(), RetryClass::None);
}

#[test]
fn update_parser_falls_back_to_unknown_variant() {
    let raw_update = json!({
        "update_type": "future_update_kind",
        "timestamp": 1_700_000_000,
        "payload": {"message_id": "m1"}
    });

    let parsed = parse_update(raw_update.clone(), UpdateSource::Webhook).expect("update should parse");

    match parsed.update_type {
        UpdateType::Unknown(value) => assert_eq!(value, "future_update_kind"),
        UpdateType::Known(other) => panic!("expected unknown update type, got {other:?}"),
    }
    assert_eq!(parsed.timestamp, 1_700_000_000);
    assert_eq!(parsed.raw, raw_update);
    assert_eq!(
        parsed.payload.get("payload"),
        Some(&json!({"message_id": "m1"}))
    );
}

#[test]
fn config_validation_rejects_zero_timeout() {
    let mut config = ClientConfig::default();
    config.request_timeout = Duration::ZERO;

    let error = config
        .validate()
        .expect_err("zero timeout must fail validation");
    assert!(matches!(
        error,
        ClientConfigValidationError::ZeroRequestTimeout
    ));
}
