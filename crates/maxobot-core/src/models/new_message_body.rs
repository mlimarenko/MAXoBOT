//! Outbound message body model and validation helpers.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::attachment::{Attachment, AttachmentValidationError, LinkMetadata};

/// Maximum supported message text length in characters.
pub const MAX_TEXT_LENGTH: usize = 4_000;

/// Supported outbound text formatting modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageTextFormat {
    /// Markdown syntax.
    Markdown,
    /// HTML syntax.
    Html,
}

/// Outbound message request body for send/edit operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewMessageBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attachments: Vec<Attachment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    link: Option<LinkMetadata>,
    #[serde(default = "default_notify", skip_serializing_if = "is_default_notify")]
    notify: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    format: Option<MessageTextFormat>,
}

/// Validation failures for [`NewMessageBody`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NewMessageBodyValidationError {
    /// Request body has no effective content.
    #[error("message body must include text, attachments, or link")]
    EmptyBody,

    /// `text` is present but blank.
    #[error("message text must not be empty")]
    EmptyText,

    /// `text` exceeds `MAX_TEXT_LENGTH`.
    #[error("message text must be at most {MAX_TEXT_LENGTH} characters, got {length}")]
    TextTooLong {
        /// Number of characters in provided text.
        length: usize,
    },

    /// `format` requires non-empty text.
    #[error("message format can only be set when non-empty text is provided")]
    FormatWithoutText,

    /// Link payload exists but URL is missing or blank.
    #[error("link payload must include non-empty url")]
    LinkWithoutUrl,

    /// One of attachments failed validation.
    #[error("attachment at index {index} is invalid: {source}")]
    InvalidAttachment {
        /// Zero-based invalid attachment index.
        index: usize,
        /// Nested attachment validation failure.
        #[source]
        source: AttachmentValidationError,
    },
}

impl Default for NewMessageBody {
    fn default() -> Self {
        Self {
            text: None,
            attachments: Vec::new(),
            link: None,
            notify: default_notify(),
            format: None,
        }
    }
}

impl NewMessageBody {
    /// Creates an empty draft body with `notify=true`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns body text.
    pub fn text(&self) -> Option<&str> {
        non_empty(self.text.as_deref())
    }

    /// Returns outbound attachments.
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }

    /// Returns optional link metadata.
    pub fn link(&self) -> Option<&LinkMetadata> {
        self.link.as_ref()
    }

    /// Returns notification flag.
    pub fn notify(&self) -> bool {
        self.notify
    }

    /// Returns text formatting mode.
    pub fn format(&self) -> Option<MessageTextFormat> {
        self.format
    }

    /// Sets text value.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Appends one outbound attachment.
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Replaces attachments list.
    pub fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }

    /// Sets rich link metadata.
    pub fn with_link(mut self, link: LinkMetadata) -> Self {
        self.link = Some(link);
        self
    }

    /// Sets notification mode.
    pub fn with_notify(mut self, notify: bool) -> Self {
        self.notify = notify;
        self
    }

    /// Sets text format.
    pub fn with_format(mut self, format: MessageTextFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Validates body fields against MAX request constraints.
    pub fn validate(&self) -> Result<(), NewMessageBodyValidationError> {
        let has_text = if let Some(text) = self.text.as_deref() {
            if text.trim().is_empty() {
                return Err(NewMessageBodyValidationError::EmptyText);
            }

            let length = text.chars().count();
            if length > MAX_TEXT_LENGTH {
                return Err(NewMessageBodyValidationError::TextTooLong { length });
            }

            true
        } else {
            false
        };

        if self.format.is_some() && !has_text {
            return Err(NewMessageBodyValidationError::FormatWithoutText);
        }

        if let Some(link) = self.link.as_ref()
            && link.url().is_none()
        {
            return Err(NewMessageBodyValidationError::LinkWithoutUrl);
        }

        for (index, attachment) in self.attachments.iter().enumerate() {
            attachment.validate_for_outbound().map_err(|source| {
                NewMessageBodyValidationError::InvalidAttachment { index, source }
            })?;
        }

        if !has_text && self.attachments.is_empty() && self.link.is_none() {
            return Err(NewMessageBodyValidationError::EmptyBody);
        }

        Ok(())
    }

    /// Returns validated body or error.
    pub fn validated(self) -> Result<Self, NewMessageBodyValidationError> {
        self.validate()?;
        Ok(self)
    }
}

fn default_notify() -> bool {
    true
}

fn is_default_notify(value: &bool) -> bool {
    *value
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_TEXT_LENGTH, MessageTextFormat, NewMessageBody, NewMessageBodyValidationError,
    };
    use crate::models::attachment::{Attachment, KnownAttachmentType, LinkMetadata};

    #[test]
    fn empty_body_is_rejected() {
        let body = NewMessageBody::new();
        assert_eq!(
            body.validate().expect_err("body should be invalid"),
            NewMessageBodyValidationError::EmptyBody
        );
    }

    #[test]
    fn format_requires_non_empty_text() {
        let body = NewMessageBody::new().with_format(MessageTextFormat::Markdown);
        assert_eq!(
            body.validate()
                .expect_err("format without text should fail"),
            NewMessageBodyValidationError::FormatWithoutText
        );
    }

    #[test]
    fn text_length_limit_is_enforced() {
        let long_text: String = "a".repeat(MAX_TEXT_LENGTH + 1);
        let body = NewMessageBody::new().with_text(long_text);

        assert!(matches!(
            body.validate().expect_err("long text should fail"),
            NewMessageBodyValidationError::TextTooLong { .. }
        ));
    }

    #[test]
    fn attachment_payload_is_validated() {
        let invalid_attachment: Attachment =
            serde_json::from_str(r#"{"type":"image","payload":{}}"#).expect("should parse");
        let body = NewMessageBody::new().with_attachment(invalid_attachment);

        assert!(matches!(
            body.validate().expect_err("attachment should fail"),
            NewMessageBodyValidationError::InvalidAttachment { .. }
        ));
    }

    #[test]
    fn valid_text_and_attachment_body_passes() {
        let body = NewMessageBody::new()
            .with_text("hello")
            .with_format(MessageTextFormat::Markdown)
            .with_attachment(Attachment::media_token(KnownAttachmentType::Image, "token"));

        body.validate().expect("body should validate");
    }

    #[test]
    fn link_requires_non_empty_url() {
        let bad_link = serde_json::from_value::<LinkMetadata>(serde_json::json!({"title":"x"}))
            .expect("should parse metadata");

        let body = NewMessageBody::new().with_link(bad_link);
        assert_eq!(
            body.validate().expect_err("link should fail"),
            NewMessageBodyValidationError::LinkWithoutUrl
        );
    }

    #[test]
    fn notify_defaults_to_true() {
        let body = NewMessageBody::new().with_text("hello");
        assert!(body.notify());

        let encoded = serde_json::to_value(&body).expect("should serialize");
        assert_eq!(encoded.get("notify"), None);
    }
}
