//! Adapter runtime configuration.

/// Unknown update handling policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownEventPolicy {
    /// Emit `interaction.channel.unknown` events.
    #[default]
    EmitAsUnknown,
    /// Ignore unknown updates with warning.
    DropWithWarning,
    /// Fail fast on unknown updates.
    FailFast,
}

/// MAX-to-Botron adapter configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterConfig {
    /// Enables strict mapping checks and fail-fast behavior.
    pub strict_mode: bool,
    /// Includes raw payload in mapped events.
    pub include_raw_payload: bool,
    /// Unknown update handling policy.
    pub unknown_event_policy: UnknownEventPolicy,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            include_raw_payload: true,
            unknown_event_policy: UnknownEventPolicy::default(),
        }
    }
}
