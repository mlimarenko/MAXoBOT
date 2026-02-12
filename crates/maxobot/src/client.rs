//! Public client facade and convenience constructors.

use maxobot_core::{
    api::client::BotApiClient, auth::credentials::BotCredentials,
    client::http_executor::ReqwestHttpExecutor, config::client_config::ClientConfig,
    errors::api_error::ApiError,
};

/// Reqwest-backed typed API client.
pub type ReqwestBotClient = BotApiClient<ReqwestHttpExecutor>;

/// Creates reqwest-backed client with timeout from [`ClientConfig`].
pub fn new_reqwest_bot_client(
    config: ClientConfig,
    credentials: BotCredentials,
) -> Result<ReqwestBotClient, ApiError> {
    let timeout = config.request_timeout;
    let client = reqwest::Client::builder().timeout(timeout).build()?;
    let executor = ReqwestHttpExecutor::new(client);
    BotApiClient::new(executor, config, credentials)
}

/// Re-export of generic typed client.
pub use maxobot_core::api::client::BotApiClient as GenericBotApiClient;
