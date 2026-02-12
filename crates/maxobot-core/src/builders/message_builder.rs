//! Fluent outbound message builder.

use crate::models::{
    attachment::{Attachment, LinkMetadata},
    new_message_body::{MessageTextFormat, NewMessageBody, NewMessageBodyValidationError},
};

/// Fluent builder for [`NewMessageBody`].
#[derive(Debug, Clone, Default)]
pub struct MessageBuilder {
    body: NewMessageBody,
}

impl MessageBuilder {
    /// Creates a new message builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            body: NewMessageBody::new(),
        }
    }

    /// Sets plain text.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.body = self.body.with_text(text.into());
        self
    }

    /// Sets markdown text with format marker.
    #[must_use]
    pub fn markdown(mut self, text: impl Into<String>) -> Self {
        self.body = self
            .body
            .with_text(text.into())
            .with_format(MessageTextFormat::Markdown);
        self
    }

    /// Sets html text with format marker.
    #[must_use]
    pub fn html(mut self, text: impl Into<String>) -> Self {
        self.body = self
            .body
            .with_text(text.into())
            .with_format(MessageTextFormat::Html);
        self
    }

    /// Adds one attachment.
    #[must_use]
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.body = self.body.with_attachment(attachment);
        self
    }

    /// Adds many attachments.
    #[must_use]
    pub fn attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.body = self.body.with_attachments(attachments);
        self
    }

    /// Sets link metadata.
    #[must_use]
    pub fn link(mut self, link: LinkMetadata) -> Self {
        self.body = self.body.with_link(link);
        self
    }

    /// Sets notification behavior.
    #[must_use]
    pub fn notify(mut self, notify: bool) -> Self {
        self.body = self.body.with_notify(notify);
        self
    }

    /// Builds and validates outbound message body.
    pub fn build(self) -> Result<NewMessageBody, NewMessageBodyValidationError> {
        self.body.validated()
    }
}

#[cfg(test)]
mod tests {
    use crate::models::attachment::{Attachment, KnownAttachmentType};

    use super::MessageBuilder;

    #[test]
    fn builds_markdown_body() {
        let body = MessageBuilder::new()
            .markdown("hello")
            .build()
            .expect("message should validate");
        assert_eq!(body.text(), Some("hello"));
    }

    #[test]
    fn builds_attachment_body() {
        let body = MessageBuilder::new()
            .attachment(Attachment::media_token(KnownAttachmentType::Image, "t1"))
            .build()
            .expect("message should validate");
        assert_eq!(body.attachments().len(), 1);
    }
}
