//! Outbound action mapping from Botron capability requests.

use async_trait::async_trait;
use serde::Serialize;

use maxobot_core::{
    api::{
        callback_answers::CallbackAnswerRequest,
        chats_actions::{PinMessageRequest, SendActionRequest},
        messages_edit::EditMessageRequest,
        messages_send::SendMessageRequest,
    },
    client::http_executor::HttpExecutor,
    errors::api_error::ApiError,
    models::new_message_body::NewMessageBody,
};

/// Botron outbound action mapped to MAX SDK call.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum BotronAction {
    /// `interaction.message.send`
    MessageSend {
        /// Optional target chat ID.
        chat_id: Option<i64>,
        /// Optional target user ID.
        user_id: Option<i64>,
        /// Message payload.
        body: NewMessageBody,
    },
    /// `interaction.message.edit`
    MessageEdit {
        /// MAX message ID.
        message_id: String,
        /// Updated payload.
        body: NewMessageBody,
    },
    /// `interaction.message.delete`
    MessageDelete {
        /// MAX message ID.
        message_id: String,
    },
    /// `interaction.callback.answer`
    CallbackAnswer {
        /// MAX callback ID.
        callback_id: String,
        /// Callback answer payload.
        answer: CallbackAnswerRequest,
    },
    /// `interaction.chat.action`
    ChatAction {
        /// MAX chat ID.
        chat_id: i64,
        /// Action string accepted by MAX API.
        action: String,
    },
    /// `interaction.chat.pin`
    ChatPin {
        /// MAX chat ID.
        chat_id: i64,
        /// Message ID to pin.
        message_id: String,
    },
}

/// Executor contract used by outbound action mapper.
#[async_trait]
pub trait MaxActionExecutor: Send + Sync {
    /// Executes send-message call.
    async fn send_message(
        &self,
        chat_id: Option<i64>,
        user_id: Option<i64>,
        body: NewMessageBody,
    ) -> Result<(), ApiError>;

    /// Executes edit-message call.
    async fn edit_message(&self, message_id: String, body: NewMessageBody) -> Result<(), ApiError>;

    /// Executes delete-message call.
    async fn delete_message(&self, message_id: String) -> Result<(), ApiError>;

    /// Executes callback answer call.
    async fn answer_callback(
        &self,
        callback_id: String,
        answer: CallbackAnswerRequest,
    ) -> Result<(), ApiError>;

    /// Executes chat action call.
    async fn send_chat_action(&self, chat_id: i64, action: String) -> Result<(), ApiError>;

    /// Executes pin-message call.
    async fn pin_chat_message(&self, chat_id: i64, message_id: String) -> Result<(), ApiError>;
}

#[async_trait]
impl<E> MaxActionExecutor for maxobot_core::api::client::BotApiClient<E>
where
    E: HttpExecutor,
{
    async fn send_message(
        &self,
        chat_id: Option<i64>,
        user_id: Option<i64>,
        body: NewMessageBody,
    ) -> Result<(), ApiError> {
        let request = match (chat_id, user_id) {
            (Some(chat_id), None) => SendMessageRequest::to_chat(chat_id, body),
            (None, Some(user_id)) => SendMessageRequest::to_user(user_id, body),
            _ => {
                return Err(ApiError::InvalidConfiguration(
                    "message send requires exactly one recipient: chat_id xor user_id".to_owned(),
                ));
            }
        };
        self.send_message(&request).await.map(|_| ())
    }

    async fn edit_message(&self, message_id: String, body: NewMessageBody) -> Result<(), ApiError> {
        let request = EditMessageRequest::new(message_id, body);
        self.edit_message(&request).await.map(|_| ())
    }

    async fn delete_message(&self, message_id: String) -> Result<(), ApiError> {
        self.delete_message(&message_id).await.map(|_| ())
    }

    async fn answer_callback(
        &self,
        callback_id: String,
        answer: CallbackAnswerRequest,
    ) -> Result<(), ApiError> {
        self.answer_callback(&callback_id, &answer)
            .await
            .map(|_| ())
    }

    async fn send_chat_action(&self, chat_id: i64, action: String) -> Result<(), ApiError> {
        let request = SendActionRequest::new(action)?;
        self.send_action(chat_id, &request).await.map(|_| ())
    }

    async fn pin_chat_message(&self, chat_id: i64, message_id: String) -> Result<(), ApiError> {
        let request = PinMessageRequest::new(message_id)?;
        self.pin_message(chat_id, &request).await.map(|_| ())
    }
}

/// Maps outbound Botron action into MAX SDK operation call.
pub async fn execute_outbound_action<E>(executor: &E, action: BotronAction) -> Result<(), ApiError>
where
    E: MaxActionExecutor,
{
    match action {
        BotronAction::MessageSend {
            chat_id,
            user_id,
            body,
        } => executor.send_message(chat_id, user_id, body).await,
        BotronAction::MessageEdit { message_id, body } => {
            executor.edit_message(message_id, body).await
        }
        BotronAction::MessageDelete { message_id } => executor.delete_message(message_id).await,
        BotronAction::CallbackAnswer {
            callback_id,
            answer,
        } => executor.answer_callback(callback_id, answer).await,
        BotronAction::ChatAction { chat_id, action } => {
            executor.send_chat_action(chat_id, action).await
        }
        BotronAction::ChatPin {
            chat_id,
            message_id,
        } => executor.pin_chat_message(chat_id, message_id).await,
    }
}
