#![allow(unused_crate_dependencies)]

//! Botron adapter smoke example.
//!
//! Simulates one inbound MAX callback update mapping and one outbound callback answer action.

use async_trait::async_trait;
use maxobot_botron_adapter::{
    AdapterConfig, AdapterContext, BotronAction, MaxActionExecutor, execute_outbound_action,
    map_inbound_update,
};
use maxobot_core::{
    api::callback_answers::CallbackAnswerRequest,
    errors::api_error::ApiError,
    models::new_message_body::NewMessageBody,
    updates::update_envelope::{KnownUpdateType, UpdateEnvelope, UpdateSource, UpdateType},
};
use serde_json::json;

#[derive(Debug, Default)]
struct StdoutExecutor;

#[async_trait]
impl MaxActionExecutor for StdoutExecutor {
    async fn send_message(
        &self,
        chat_id: Option<i64>,
        user_id: Option<i64>,
        _body: NewMessageBody,
    ) -> Result<(), ApiError> {
        println!("send_message => chat_id={chat_id:?}, user_id={user_id:?}");
        Ok(())
    }

    async fn edit_message(
        &self,
        message_id: String,
        _body: NewMessageBody,
    ) -> Result<(), ApiError> {
        println!("edit_message => message_id={message_id}");
        Ok(())
    }

    async fn delete_message(&self, message_id: String) -> Result<(), ApiError> {
        println!("delete_message => message_id={message_id}");
        Ok(())
    }

    async fn answer_callback(
        &self,
        callback_id: String,
        _answer: CallbackAnswerRequest,
    ) -> Result<(), ApiError> {
        println!("answer_callback => callback_id={callback_id}");
        Ok(())
    }

    async fn send_chat_action(&self, chat_id: i64, action: String) -> Result<(), ApiError> {
        println!("send_chat_action => chat_id={chat_id}, action={action}");
        Ok(())
    }

    async fn pin_chat_message(&self, chat_id: i64, message_id: String) -> Result<(), ApiError> {
        println!("pin_chat_message => chat_id={chat_id}, message_id={message_id}");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let update = UpdateEnvelope {
        update_type: UpdateType::Known(KnownUpdateType::MessageCallback),
        timestamp: 1_700_000_300_000_i64,
        payload: json!({
            "payload": {
                "chat_id": 77,
                "user_id": 42,
                "message_id": "m-100",
                "callback_id": "cb-100",
                "payload": {"button": "approve"}
            }
        }),
        raw: json!({}),
        source: UpdateSource::Webhook,
    };

    let inbound = map_inbound_update(&update, AdapterContext::default(), &AdapterConfig::default())?
        .expect("callback update should map");
    println!("mapped inbound event: {}", inbound.event_name);

    execute_outbound_action(
        &StdoutExecutor,
        BotronAction::CallbackAnswer {
            callback_id: "cb-100".to_owned(),
            answer: CallbackAnswerRequest::new().with_notification("Approved"),
        },
    )
    .await?;

    Ok(())
}
