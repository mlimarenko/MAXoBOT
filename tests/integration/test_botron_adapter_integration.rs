use std::sync::{Arc, Mutex};

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

#[derive(Debug, Clone, Default)]
struct MockExecutor {
    calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl MaxActionExecutor for MockExecutor {
    async fn send_message(
        &self,
        chat_id: Option<i64>,
        user_id: Option<i64>,
        _body: NewMessageBody,
    ) -> Result<(), ApiError> {
        self.calls
            .lock()
            .expect("lock should not be poisoned")
            .push(format!("send:{chat_id:?}:{user_id:?}"));
        Ok(())
    }

    async fn edit_message(
        &self,
        message_id: String,
        _body: NewMessageBody,
    ) -> Result<(), ApiError> {
        self.calls
            .lock()
            .expect("lock should not be poisoned")
            .push(format!("edit:{message_id}"));
        Ok(())
    }

    async fn delete_message(&self, message_id: String) -> Result<(), ApiError> {
        self.calls
            .lock()
            .expect("lock should not be poisoned")
            .push(format!("delete:{message_id}"));
        Ok(())
    }

    async fn answer_callback(
        &self,
        callback_id: String,
        _answer: CallbackAnswerRequest,
    ) -> Result<(), ApiError> {
        self.calls
            .lock()
            .expect("lock should not be poisoned")
            .push(format!("answer:{callback_id}"));
        Ok(())
    }

    async fn send_chat_action(&self, chat_id: i64, action: String) -> Result<(), ApiError> {
        self.calls
            .lock()
            .expect("lock should not be poisoned")
            .push(format!("action:{chat_id}:{action}"));
        Ok(())
    }

    async fn pin_chat_message(&self, chat_id: i64, message_id: String) -> Result<(), ApiError> {
        self.calls
            .lock()
            .expect("lock should not be poisoned")
            .push(format!("pin:{chat_id}:{message_id}"));
        Ok(())
    }
}

#[tokio::test]
async fn adapter_maps_inbound_callback_and_executes_outbound_response() {
    let update = UpdateEnvelope {
        update_type: UpdateType::Known(KnownUpdateType::MessageCallback),
        timestamp: 1_700_000_000_777_i64,
        payload: json!({
            "payload": {
                "chat_id": 100,
                "user_id": 200,
                "message_id": "m-10",
                "callback_id": "cb-10",
                "payload": {"button": "approve"}
            }
        }),
        raw: json!({}),
        source: UpdateSource::Webhook,
    };

    let inbound = map_inbound_update(&update, AdapterContext::default(), &AdapterConfig::default())
        .expect("inbound mapping should succeed")
        .expect("callback update should map to event");

    assert_eq!(inbound.event_name, "interaction.callback.received");
    assert_eq!(inbound.context.external_ids.callback_id.as_deref(), Some("cb-10"));

    let executor = MockExecutor::default();
    execute_outbound_action(
        &executor,
        BotronAction::CallbackAnswer {
            callback_id: "cb-10".to_owned(),
            answer: CallbackAnswerRequest::new().with_notification("approved"),
        },
    )
    .await
    .expect("outbound callback answer should succeed");

    let calls = executor
        .calls
        .lock()
        .expect("lock should not be poisoned")
        .clone();
    assert_eq!(calls, vec!["answer:cb-10".to_owned()]);
}
