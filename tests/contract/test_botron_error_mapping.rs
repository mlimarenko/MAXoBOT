use http::StatusCode;

use maxobot_botron_adapter::{BotronFailureClass, translate_api_error};
use maxobot_core::errors::api_error::ApiError;

#[test]
fn maps_sdk_error_classes_to_botron_failure_classes() {
    let auth = ApiError::from_status(StatusCode::UNAUTHORIZED, None, None);
    let rate_limited = ApiError::from_status(StatusCode::TOO_MANY_REQUESTS, None, None);
    let unavailable = ApiError::from_status(StatusCode::SERVICE_UNAVAILABLE, None, None);
    let contract = ApiError::InvalidConfiguration("bad payload".to_owned());

    assert_eq!(translate_api_error(&auth), BotronFailureClass::ChannelAuthFailed);
    assert_eq!(
        translate_api_error(&rate_limited),
        BotronFailureClass::ChannelRateLimited
    );
    assert_eq!(
        translate_api_error(&unavailable),
        BotronFailureClass::ChannelProviderUnavailable
    );
    assert_eq!(
        translate_api_error(&contract),
        BotronFailureClass::ChannelContractError
    );
}
