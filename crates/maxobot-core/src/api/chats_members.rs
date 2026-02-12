//! Chat membership and admin operations.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::{action_result::ActionResult, chat_membership::ChatMembership, user::User},
};

/// Request for `POST /chats/{chatId}/members/admins`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddAdminRequest {
    /// User ID that should become admin.
    pub user_id: i64,
}

/// Request for `POST /chats/{chatId}/members`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddMembersRequest {
    /// User IDs that should be added to chat.
    pub user_ids: Vec<i64>,
}

impl AddMembersRequest {
    /// Creates request and validates non-empty user list.
    pub fn new(user_ids: Vec<i64>) -> Result<Self, ApiError> {
        if user_ids.is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "user_ids must not be empty".to_owned(),
            ));
        }
        Ok(Self { user_ids })
    }
}

/// Request for `DELETE /chats/{chatId}/members`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoveMemberRequest {
    /// User ID that should be removed from chat.
    pub user_id: i64,
}

#[derive(Debug, Deserialize)]
struct AdminsEnvelope {
    #[serde(default)]
    admins: Vec<User>,
}

#[derive(Debug, Deserialize)]
struct MembersEnvelope {
    #[serde(default)]
    members: Vec<User>,
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Gets bot membership via `GET /chats/{chatId}/members/me`.
    pub async fn get_bot_membership(&self, chat_id: i64) -> Result<ChatMembership, ApiError> {
        let request = self
            .request_builder(Method::GET, api_paths::chat_members_me(chat_id))?
            .build()?;
        self.execute_json(request).await
    }

    /// Leaves chat via `DELETE /chats/{chatId}/members/me`.
    pub async fn leave_chat(&self, chat_id: i64) -> Result<ActionResult, ApiError> {
        let request = self
            .request_builder(Method::DELETE, api_paths::chat_members_me(chat_id))?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }

    /// Lists chat admins via `GET /chats/{chatId}/members/admins`.
    pub async fn get_admins(&self, chat_id: i64) -> Result<Vec<User>, ApiError> {
        let request = self
            .request_builder(Method::GET, api_paths::chat_members_admins(chat_id))?
            .build()?;
        let envelope: AdminsEnvelope = self.execute_json(request).await?;
        Ok(envelope.admins)
    }

    /// Adds admin via `POST /chats/{chatId}/members/admins`.
    pub async fn add_admin(
        &self,
        chat_id: i64,
        request: &AddAdminRequest,
    ) -> Result<ActionResult, ApiError> {
        let request = self
            .request_builder(Method::POST, api_paths::chat_members_admins(chat_id))?
            .with_body(request)?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }

    /// Removes admin via `DELETE /chats/{chatId}/members/admins/{userId}`.
    pub async fn remove_admin(&self, chat_id: i64, user_id: i64) -> Result<ActionResult, ApiError> {
        let request = self
            .request_builder(
                Method::DELETE,
                api_paths::chat_members_admin(chat_id, user_id),
            )?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }

    /// Lists chat members via `GET /chats/{chatId}/members`.
    pub async fn get_members(&self, chat_id: i64) -> Result<Vec<User>, ApiError> {
        let request = self
            .request_builder(Method::GET, api_paths::chat_members(chat_id))?
            .build()?;
        let envelope: MembersEnvelope = self.execute_json(request).await?;
        Ok(envelope.members)
    }

    /// Adds members via `POST /chats/{chatId}/members`.
    pub async fn add_members(
        &self,
        chat_id: i64,
        request: &AddMembersRequest,
    ) -> Result<ActionResult, ApiError> {
        if request.user_ids.is_empty() {
            return Err(ApiError::InvalidConfiguration(
                "user_ids must not be empty".to_owned(),
            ));
        }

        let request = self
            .request_builder(Method::POST, api_paths::chat_members(chat_id))?
            .with_body(request)?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }

    /// Removes member via `DELETE /chats/{chatId}/members`.
    pub async fn remove_member(
        &self,
        chat_id: i64,
        request: &RemoveMemberRequest,
    ) -> Result<ActionResult, ApiError> {
        let request = self
            .request_builder(Method::DELETE, api_paths::chat_members(chat_id))?
            .with_body(request)?
            .build()?;
        let result = self.execute_optional_json::<ActionResult>(request).await?;
        Ok(result.unwrap_or_else(ActionResult::success))
    }
}
