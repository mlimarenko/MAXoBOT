#![allow(unused_crate_dependencies)]

//! Typed polling echo example.
//!
//! This example demonstrates SDK usage for send/reply/upload primitives.

use maxobot_core::{
    api::{client::BotApiClient, messages_send::SendMessageRequest, uploads::UploadType},
    auth::credentials::BotCredentials,
    builders::message_builder::MessageBuilder,
    client::http_executor::ReqwestHttpExecutor,
    config::client_config::ClientConfig,
    models::message_helpers::MessageExt,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("MAX_BOT_TOKEN").unwrap_or_else(|_| "replace-me".to_owned());

    let credentials = BotCredentials::new(token)?;
    let config = ClientConfig::default();
    let reqwest_client = reqwest::Client::builder()
        .timeout(config.request_timeout)
        .build()?;
    let executor = ReqwestHttpExecutor::new(reqwest_client);
    let client = BotApiClient::new(executor, config, credentials)?;

    let outbound = MessageBuilder::new().markdown("Echo bootstrap").build()?;
    let sent = client
        .send_message(&SendMessageRequest::to_chat(1, outbound))
        .await?;

    let _upload_ticket = client.create_upload_ticket(UploadType::Image).await?;

    let reply_body = MessageBuilder::new().text("Reply from helper").build()?;
    sent.bind(&client)?.reply(reply_body).await?;

    Ok(())
}
