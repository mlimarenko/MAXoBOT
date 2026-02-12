#![allow(unused_crate_dependencies)]

//! Webhook echo example with verifier + dispatcher integration.
//!
//! Environment variables:
//! - `MAX_BOT_TOKEN` (required)
//! - `MAX_WEBHOOK_SECRET` (optional)
//! - `MAX_WEBHOOK_ADDR` (optional, default `127.0.0.1:8080`)

use std::{net::SocketAddr, sync::Arc};

use maxobot::{
    client::{ReqwestBotClient, new_reqwest_bot_client},
    prelude::{BotCredentials, ClientConfig},
};
use maxobot_core::{
    api::callback_answers::CallbackAnswerRequest, updates::update_envelope::KnownUpdateType,
};
use maxobot_dispatch::{
    DispatchContext, DispatchError, SequentialDispatcher,
    router::{Router, UpdateSelector, shared_handler},
};
use maxobot_core::updates::update_envelope::UpdateEnvelope;
use maxobot_webhook::{
    axum_adapter::webhook_router_with_dispatcher, verifier::WebhookVerifier,
};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("MAX_BOT_TOKEN")
        .map_err(|_| "MAX_BOT_TOKEN is required for webhook example")?;
    let secret = std::env::var("MAX_WEBHOOK_SECRET").ok();
    let bind_addr = std::env::var("MAX_WEBHOOK_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_owned())
        .parse::<SocketAddr>()?;

    let credentials = BotCredentials::new(token)?;
    let client = Arc::new(new_reqwest_bot_client(ClientConfig::default(), credentials)?);
    let dispatcher = Arc::new(build_dispatcher(Arc::clone(&client)));

    let app = webhook_router_with_dispatcher(WebhookVerifier::new(secret), dispatcher);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    println!("MAX webhook echo example listening on http://{bind_addr}/webhook");
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_dispatcher(client: Arc<ReqwestBotClient>) -> SequentialDispatcher {
    let mut router = Router::new();
    router.register(
        UpdateSelector::Known(KnownUpdateType::MessageCallback),
        shared_handler(move |update: UpdateEnvelope, _context: DispatchContext| {
            let client = Arc::clone(&client);
            async move {
                if let Some(callback_id) = extract_callback_id(&update.payload) {
                    let request = CallbackAnswerRequest::new().with_notification("Callback received");
                    client
                        .answer_callback(callback_id, &request)
                        .await
                        .map(|_| ())
                        .map_err(|error| DispatchError::Handler(error.to_string()))?;
                }
                Ok(())
            }
        }),
    );
    SequentialDispatcher::new(router)
}

fn extract_callback_id(payload: &Value) -> Option<&str> {
    payload
        .get("payload")
        .and_then(|payload| payload.get("callback_id"))
        .and_then(Value::as_str)
}
