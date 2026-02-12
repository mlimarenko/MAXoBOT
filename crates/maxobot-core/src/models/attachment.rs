//! Attachment models used by inbound and outbound message payloads.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use thiserror::Error;

/// Supported built-in attachment type names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownAttachmentType {
    /// Image attachment payload.
    Image,
    /// Video attachment payload.
    Video,
    /// Audio attachment payload.
    Audio,
    /// Generic file attachment payload.
    File,
    /// Rich link preview payload.
    Link,
}

impl KnownAttachmentType {
    /// Returns canonical API string value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::File => "file",
            Self::Link => "link",
        }
    }
}

/// Attachment type that keeps unknown values for forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttachmentType {
    /// Known attachment type.
    Known(KnownAttachmentType),
    /// Unknown/custom attachment type from API.
    Custom(String),
}

impl AttachmentType {
    /// Returns the type as string value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(kind) => kind.as_str(),
            Self::Custom(kind) => kind.as_str(),
        }
    }
}

impl From<KnownAttachmentType> for AttachmentType {
    fn from(value: KnownAttachmentType) -> Self {
        Self::Known(value)
    }
}

impl From<String> for AttachmentType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "image" => Self::Known(KnownAttachmentType::Image),
            "video" => Self::Known(KnownAttachmentType::Video),
            "audio" => Self::Known(KnownAttachmentType::Audio),
            "file" => Self::Known(KnownAttachmentType::File),
            "link" => Self::Known(KnownAttachmentType::Link),
            _ => Self::Custom(value),
        }
    }
}

impl From<&str> for AttachmentType {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

/// Typed view of attachment payload.
#[derive(Debug, Clone, PartialEq)]
pub enum AttachmentPayload {
    /// Media payload with upload token and optional metadata.
    Media(AttachmentFilePayload),
    /// Link payload with preview metadata.
    Link(LinkMetadata),
    /// Unknown payload kept as raw JSON.
    Unknown(Value),
}

/// MAX attachment entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    #[serde(rename = "type")]
    kind: AttachmentType,
    #[serde(default)]
    payload: Value,
}

/// Errors returned by [`Attachment::validate_for_outbound`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AttachmentValidationError {
    /// Attachment type is not currently supported for outbound payloads.
    #[error("unsupported attachment type '{attachment_type}'")]
    UnsupportedAttachmentType {
        /// Invalid type string.
        attachment_type: String,
    },

    /// Media payload does not include required token.
    #[error("attachment type '{attachment_type}' must include non-empty payload token")]
    MissingMediaToken {
        /// Media attachment type.
        attachment_type: String,
    },

    /// Link payload does not include URL.
    #[error("link attachment payload must include non-empty url")]
    MissingLinkUrl,
}

impl Attachment {
    /// Creates attachment from explicit type and raw payload JSON.
    pub fn new(kind: impl Into<AttachmentType>, payload: Value) -> Self {
        Self {
            kind: kind.into(),
            payload,
        }
    }

    /// Creates media attachment payload from upload token.
    pub fn media_token(kind: KnownAttachmentType, token: impl Into<String>) -> Self {
        Self::new(kind, json!({ "token": token.into() }))
    }

    /// Creates link attachment payload from structured metadata.
    pub fn link(link: LinkMetadata) -> Self {
        let payload = serde_json::to_value(link).unwrap_or(Value::Null);
        Self::new(KnownAttachmentType::Link, payload)
    }

    /// Returns attachment type descriptor.
    pub fn kind(&self) -> &AttachmentType {
        &self.kind
    }

    /// Returns raw payload JSON.
    pub fn raw_payload(&self) -> &Value {
        &self.payload
    }

    /// Returns typed payload representation.
    pub fn payload(&self) -> AttachmentPayload {
        match self.kind.as_str() {
            "image" | "video" | "audio" | "file" => {
                serde_json::from_value::<AttachmentFilePayload>(self.payload.clone())
                    .map(AttachmentPayload::Media)
                    .unwrap_or_else(|_| AttachmentPayload::Unknown(self.payload.clone()))
            }
            "link" => serde_json::from_value::<LinkMetadata>(self.payload.clone())
                .map(AttachmentPayload::Link)
                .unwrap_or_else(|_| AttachmentPayload::Unknown(self.payload.clone())),
            _ => AttachmentPayload::Unknown(self.payload.clone()),
        }
    }

    /// Validates outbound attachment payload for known SDK-supported types.
    pub fn validate_for_outbound(&self) -> Result<(), AttachmentValidationError> {
        let attachment_type = self.kind.as_str();

        match attachment_type {
            "image" | "video" | "audio" | "file" => match self.payload() {
                AttachmentPayload::Media(media) if media.token().is_some() => Ok(()),
                _ => Err(AttachmentValidationError::MissingMediaToken {
                    attachment_type: attachment_type.to_owned(),
                }),
            },
            "link" => match self.payload() {
                AttachmentPayload::Link(link) if link.url().is_some() => Ok(()),
                _ => Err(AttachmentValidationError::MissingLinkUrl),
            },
            _ => Err(AttachmentValidationError::UnsupportedAttachmentType {
                attachment_type: attachment_type.to_owned(),
            }),
        }
    }
}

/// Token-based attachment payload plus optional media metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttachmentFilePayload {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    duration_ms: Option<u64>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl AttachmentFilePayload {
    /// Returns upload token used by send/edit operations.
    pub fn token(&self) -> Option<&str> {
        non_empty(self.token.as_deref())
    }

    /// Returns attachment URL when API includes it.
    pub fn url(&self) -> Option<&str> {
        non_empty(self.url.as_deref())
    }

    /// Returns optional file name.
    pub fn file_name(&self) -> Option<&str> {
        non_empty(self.file_name.as_deref())
    }

    /// Returns optional MIME type.
    pub fn mime_type(&self) -> Option<&str> {
        non_empty(self.mime_type.as_deref())
    }

    /// Returns unmodeled fields captured during deserialization.
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}

/// Rich link metadata for message preview payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LinkMetadata {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    preview_url: Option<String>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl LinkMetadata {
    /// Returns link target URL.
    pub fn url(&self) -> Option<&str> {
        non_empty(self.url.as_deref())
    }

    /// Returns optional link title.
    pub fn title(&self) -> Option<&str> {
        non_empty(self.title.as_deref())
    }

    /// Returns optional link description.
    pub fn description(&self) -> Option<&str> {
        non_empty(self.description.as_deref())
    }

    /// Returns optional preview image URL.
    pub fn preview_url(&self) -> Option<&str> {
        non_empty(self.preview_url.as_deref())
    }

    /// Returns unmodeled fields captured during deserialization.
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
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
        Attachment, AttachmentPayload, AttachmentValidationError, KnownAttachmentType, LinkMetadata,
    };

    #[test]
    fn parses_media_attachment_payload() {
        let attachment: Attachment = serde_json::from_str(
            r#"{"type":"image","payload":{"token":"upload-token","mime_type":"image/png"}}"#,
        )
        .expect("should parse attachment");

        match attachment.payload() {
            AttachmentPayload::Media(payload) => {
                assert_eq!(payload.token(), Some("upload-token"));
                assert_eq!(payload.mime_type(), Some("image/png"));
            }
            other => panic!("unexpected payload variant: {other:?}"),
        }
    }

    #[test]
    fn parses_link_payload_metadata() {
        let attachment: Attachment = serde_json::from_str(
            r#"{"type":"link","payload":{"url":"https://example.com","title":"Example"}}"#,
        )
        .expect("should parse attachment");

        match attachment.payload() {
            AttachmentPayload::Link(link) => {
                assert_eq!(link.url(), Some("https://example.com"));
                assert_eq!(link.title(), Some("Example"));
            }
            other => panic!("unexpected payload variant: {other:?}"),
        }
    }

    #[test]
    fn outbound_validation_rejects_invalid_payloads() {
        let attachment: Attachment = serde_json::from_str(r#"{"type":"image","payload":{}}"#)
            .expect("should parse attachment");

        let error = attachment
            .validate_for_outbound()
            .expect_err("payload token must be required");

        assert!(matches!(
            error,
            AttachmentValidationError::MissingMediaToken { .. }
        ));
    }

    #[test]
    fn outbound_validation_accepts_media_token_and_link_url() {
        let media = Attachment::media_token(KnownAttachmentType::File, "file-token");
        media
            .validate_for_outbound()
            .expect("media token attachment should validate");

        let link = Attachment::link(
            serde_json::from_value::<LinkMetadata>(
                serde_json::json!({ "url": "https://example.com", "title": "Example" }),
            )
            .expect("link metadata should decode"),
        );
        link.validate_for_outbound()
            .expect("link attachment should validate");
    }
}
