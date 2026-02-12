//! Marker commit strategy definitions.

/// Strategy controlling when fetched marker is committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommitStrategy {
    /// Commit marker only when update handling succeeds.
    #[default]
    AfterSuccess,
    /// Never auto-commit; caller must commit manually.
    Manual,
}

impl CommitStrategy {
    /// Returns whether marker should be committed automatically for the outcome.
    #[must_use]
    pub fn should_commit(self, handled_successfully: bool) -> bool {
        match self {
            Self::AfterSuccess => handled_successfully,
            Self::Manual => false,
        }
    }
}
