//! Polling/webhook mode conflict guards.

use crate::{
    api::client::BotApiClient, client::http_executor::HttpExecutor, errors::api_error::ApiError,
};

/// Action policy for polling/subscription conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PollingModeConflictPolicy {
    /// Emit warning-style result but allow polling startup.
    #[default]
    Warn,
    /// Fail fast when active webhook subscriptions exist.
    Fail,
}

/// Result of polling mode guard check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeGuardResult {
    /// Whether polling startup is allowed for selected policy.
    pub polling_allowed: bool,
    /// Number of active webhook subscriptions discovered.
    pub active_subscriptions: usize,
    /// Warning message when conflict is present under [`PollingModeConflictPolicy::Warn`].
    pub warning: Option<String>,
}

impl ModeGuardResult {
    /// Returns whether polling/subscription conflict was detected.
    #[must_use]
    pub const fn has_conflict(&self) -> bool {
        self.active_subscriptions > 0
    }
}

/// Validates polling startup against current webhook subscription state.
pub async fn enforce_polling_mode_guard<E>(
    client: &BotApiClient<E>,
    policy: PollingModeConflictPolicy,
) -> Result<ModeGuardResult, ApiError>
where
    E: HttpExecutor,
{
    let subscriptions = client.get_subscriptions().await?;
    let active_subscriptions = subscriptions
        .iter()
        .filter(|subscription| subscription.url().is_some())
        .count();

    if active_subscriptions == 0 {
        return Ok(ModeGuardResult {
            polling_allowed: true,
            active_subscriptions,
            warning: None,
        });
    }

    let warning = format!(
        "detected {active_subscriptions} active webhook subscription(s); webhook is the production-first mode"
    );
    match policy {
        PollingModeConflictPolicy::Warn => Ok(ModeGuardResult {
            polling_allowed: true,
            active_subscriptions,
            warning: Some(warning),
        }),
        PollingModeConflictPolicy::Fail => Err(ApiError::InvalidConfiguration(warning)),
    }
}
