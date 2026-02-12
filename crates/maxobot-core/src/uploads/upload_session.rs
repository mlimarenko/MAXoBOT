//! Upload lifecycle state machine.

use std::time::Duration;

use thiserror::Error;

/// Supported media upload kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadKind {
    /// Image upload kind.
    Image,
    /// Video upload kind.
    Video,
    /// Audio upload kind.
    Audio,
    /// Generic file upload kind.
    File,
}

impl UploadKind {
    /// Returns wire value expected by `POST /uploads?type=...`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::File => "file",
        }
    }
}

/// Upload session state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadSessionState {
    /// Session has been created and awaits upload ticket request.
    Initialized,
    /// Binary transfer is in progress.
    Uploading,
    /// Binary transfer completed and server accepted content.
    Uploaded,
    /// Media token is ready for attachment usage.
    Ready,
    /// Server returned delayed media processing (`attachment.not.ready`).
    NotReadyRetry {
        /// Retry attempt count.
        attempt: u32,
        /// Suggested delay before next check.
        retry_after: Duration,
    },
    /// Session reached terminal failure state.
    Failed {
        /// Redacted failure reason.
        reason: String,
    },
}

/// Upload session model tracking lifecycle transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadSession {
    kind: UploadKind,
    upload_url: Option<String>,
    token: Option<String>,
    state: UploadSessionState,
}

/// Upload session transition failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UploadSessionError {
    /// Operation is invalid for current state.
    #[error("invalid upload state transition from {from:?} to {to}")]
    InvalidTransition {
        /// Current state.
        from: UploadSessionState,
        /// Requested target state name.
        to: &'static str,
    },
    /// Required upload URL is missing.
    #[error("upload URL is missing")]
    MissingUploadUrl,
    /// Required token is missing.
    #[error("media token is missing")]
    MissingToken,
}

impl UploadSession {
    /// Creates a new upload session.
    #[must_use]
    pub fn new(kind: UploadKind) -> Self {
        Self {
            kind,
            upload_url: None,
            token: None,
            state: UploadSessionState::Initialized,
        }
    }

    /// Returns upload kind.
    #[must_use]
    pub fn kind(&self) -> UploadKind {
        self.kind
    }

    /// Returns upload URL.
    #[must_use]
    pub fn upload_url(&self) -> Option<&str> {
        self.upload_url
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns media token.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns current session state.
    #[must_use]
    pub fn state(&self) -> &UploadSessionState {
        &self.state
    }

    /// Sets upload ticket data while session is initialized.
    pub fn set_ticket(
        &mut self,
        upload_url: impl Into<String>,
        token: Option<String>,
    ) -> Result<(), UploadSessionError> {
        if !matches!(self.state, UploadSessionState::Initialized) {
            return Err(UploadSessionError::InvalidTransition {
                from: self.state.clone(),
                to: "set_ticket",
            });
        }

        self.upload_url = Some(upload_url.into());
        self.token = token;
        Ok(())
    }

    /// Moves session to uploading state.
    pub fn mark_uploading(&mut self) -> Result<(), UploadSessionError> {
        if !matches!(self.state, UploadSessionState::Initialized) {
            return Err(UploadSessionError::InvalidTransition {
                from: self.state.clone(),
                to: "Uploading",
            });
        }
        if self.upload_url().is_none() {
            return Err(UploadSessionError::MissingUploadUrl);
        }

        self.state = UploadSessionState::Uploading;
        Ok(())
    }

    /// Marks upload as completed.
    pub fn mark_uploaded(&mut self) -> Result<(), UploadSessionError> {
        if !matches!(self.state, UploadSessionState::Uploading) {
            return Err(UploadSessionError::InvalidTransition {
                from: self.state.clone(),
                to: "Uploaded",
            });
        }

        self.state = UploadSessionState::Uploaded;
        Ok(())
    }

    /// Marks upload as ready for message attachment usage.
    pub fn mark_ready(&mut self) -> Result<(), UploadSessionError> {
        if !matches!(
            self.state,
            UploadSessionState::Uploaded | UploadSessionState::NotReadyRetry { .. }
        ) {
            return Err(UploadSessionError::InvalidTransition {
                from: self.state.clone(),
                to: "Ready",
            });
        }
        if self.token().is_none() {
            return Err(UploadSessionError::MissingToken);
        }

        self.state = UploadSessionState::Ready;
        Ok(())
    }

    /// Marks delayed processing state and schedules another readiness check.
    pub fn mark_not_ready_retry(
        &mut self,
        attempt: u32,
        retry_after: Duration,
    ) -> Result<(), UploadSessionError> {
        if !matches!(
            self.state,
            UploadSessionState::Uploaded | UploadSessionState::NotReadyRetry { .. }
        ) {
            return Err(UploadSessionError::InvalidTransition {
                from: self.state.clone(),
                to: "NotReadyRetry",
            });
        }

        self.state = UploadSessionState::NotReadyRetry {
            attempt,
            retry_after,
        };
        Ok(())
    }

    /// Moves session to terminal failed state.
    pub fn mark_failed(&mut self, reason: impl Into<String>) {
        self.state = UploadSessionState::Failed {
            reason: reason.into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{UploadKind, UploadSession, UploadSessionError, UploadSessionState};

    #[test]
    fn transitions_from_initialized_to_ready() {
        let mut session = UploadSession::new(UploadKind::Image);
        session
            .set_ticket("https://upload.example", Some("token-1".to_owned()))
            .expect("ticket should be set");
        session.mark_uploading().expect("should enter uploading");
        session.mark_uploaded().expect("should enter uploaded");
        session.mark_ready().expect("should enter ready");

        assert!(matches!(session.state(), UploadSessionState::Ready));
    }

    #[test]
    fn requires_upload_url_before_uploading() {
        let mut session = UploadSession::new(UploadKind::Video);
        let error = session
            .mark_uploading()
            .expect_err("missing ticket should fail");
        assert_eq!(error, UploadSessionError::MissingUploadUrl);
    }

    #[test]
    fn supports_not_ready_retry_transition() {
        let mut session = UploadSession::new(UploadKind::Audio);
        session
            .set_ticket("https://upload.example", Some("token-2".to_owned()))
            .expect("ticket should be set");
        session.mark_uploading().expect("uploading");
        session.mark_uploaded().expect("uploaded");
        session
            .mark_not_ready_retry(1, Duration::from_millis(200))
            .expect("not-ready transition should succeed");
        assert!(matches!(
            session.state(),
            UploadSessionState::NotReadyRetry { attempt: 1, .. }
        ));
    }
}
