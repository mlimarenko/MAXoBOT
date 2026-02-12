//! Chat membership model for membership-related endpoints.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::models::user::User;

/// Bot/user membership in a chat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChatMembership {
    #[serde(default)]
    user: Option<User>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    is_admin: Option<bool>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl ChatMembership {
    /// Returns user attached to membership when present.
    #[must_use]
    pub fn user(&self) -> Option<&User> {
        self.user.as_ref()
    }

    /// Returns membership status.
    #[must_use]
    pub fn status(&self) -> Option<&str> {
        self.status
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns membership role.
    #[must_use]
    pub fn role(&self) -> Option<&str> {
        self.role
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns whether admin flag is set.
    #[must_use]
    pub fn is_admin(&self) -> Option<bool> {
        self.is_admin
    }

    /// Returns forward-compatible extra fields.
    #[must_use]
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}
