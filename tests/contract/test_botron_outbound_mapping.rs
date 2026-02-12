use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use maxobot_botron_adapter::{BotronAction, MaxActionExecutor, execute_outbound_action};
use maxobot_core::{
    api::callback_answers::CallbackAnswerRequest,
    errors::api_error::ApiError,
    models::new_message_body::NewMessageBody,
};

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

fn calls(executor: &MockExecutor) -> Vec<String> {
    executor
        .calls
        .lock()
        .expect("lock should not be poisoned")
        .clone()
}

#[tokio::test]
async fn outbound_actions_map_to_expected_executor_calls() {
    let executor = MockExecutor::default();
    let body = NewMessageBody::new().with_text("hello");

    execute_outbound_action(
        &executor,
        BotronAction::MessageSend {
            chat_id: Some(10),
            user_id: None,
            body: body.clone(),
        },
    )
    .await
    .expect("send should succeed");
    execute_outbound_action(
        &executor,
        BotronAction::MessageEdit {
            message_id: "m-1".to_owned(),
            body,
        },
    )
    .await
    .expect("edit should succeed");
    execute_outbound_action(
        &executor,
        BotronAction::MessageDelete {
            message_id: "m-1".to_owned(),
        },
    )
    .await
    .expect("delete should succeed");
    execute_outbound_action(
        &executor,
        BotronAction::CallbackAnswer {
            callback_id: "cb-1".to_owned(),
            answer: CallbackAnswerRequest::new().with_notification("ok"),
        },
    )
    .await
    .expect("answer should succeed");
    execute_outbound_action(
        &executor,
        BotronAction::ChatAction {
            chat_id: 10,
            action: "typing".to_owned(),
        },
    )
    .await
    .expect("action should succeed");
    execute_outbound_action(
        &executor,
        BotronAction::ChatPin {
            chat_id: 10,
            message_id: "m-1".to_owned(),
        },
    )
    .await
    .expect("pin should succeed");

    assert_eq!(
        calls(&executor),
        vec![
            "send:Some(10):None".to_owned(),
            "edit:m-1".to_owned(),
            "delete:m-1".to_owned(),
            "answer:cb-1".to_owned(),
            "action:10:typing".to_owned(),
            "pin:10:m-1".to_owned(),
        ]
    );
}
