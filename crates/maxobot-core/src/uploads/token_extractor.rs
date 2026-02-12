//! Media token extraction helpers for upload and attachment workflows.
//!
//! MAX media flows can surface a token in different payload shapes depending on
//! media kind (`image`, `video`, `audio`, `file`) and API step (upload ticket,
//! upload response, attachment payload, video metadata). This module centralizes
//! token extraction so call sites can use one normalized behavior.

use serde_json::Value;

use crate::models::{attachment::Attachment, upload_ticket::UploadTicket, video::VideoMetadata};

/// Media kinds that support upload-token based attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaTokenKind {
    /// Image media kind.
    Image,
    /// Video media kind.
    Video,
    /// Audio media kind.
    Audio,
    /// Generic file media kind.
    File,
}

impl MediaTokenKind {
    /// Returns canonical MAX API string value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::File => "file",
        }
    }

    /// Parses media kind from attachment type string.
    #[must_use]
    pub fn from_attachment_type(value: &str) -> Option<Self> {
        match value {
            "image" => Some(Self::Image),
            "video" => Some(Self::Video),
            "audio" => Some(Self::Audio),
            "file" => Some(Self::File),
            _ => None,
        }
    }

    const fn token_field_candidates(self) -> &'static [&'static str] {
        match self {
            Self::Image => &[
                "token",
                "image_token",
                "imageToken",
                "photo_token",
                "photoToken",
            ],
            Self::Video => &["token", "video_token", "videoToken"],
            Self::Audio => &["token", "audio_token", "audioToken"],
            Self::File => &["token", "file_token", "fileToken"],
        }
    }

    const fn container_field_candidates(self) -> &'static [&'static str] {
        match self {
            Self::Image => &["image", "photo", "images", "photos"],
            Self::Video => &["video", "videos"],
            Self::Audio => &["audio", "audios"],
            Self::File => &["file", "files"],
        }
    }
}

/// Extracts token from attachment payload if attachment type is one of supported
/// media kinds.
#[must_use]
pub fn extract_attachment_token(attachment: &Attachment) -> Option<&str> {
    let media_kind = MediaTokenKind::from_attachment_type(attachment.kind().as_str())?;
    extract_upload_response_token(media_kind, attachment.raw_payload())
}

/// Extracts token from attachment payload for explicitly requested media kind.
///
/// Returns `None` when attachment type does not match `media_kind`.
#[must_use]
pub fn extract_attachment_token_for_kind(
    attachment: &Attachment,
    media_kind: MediaTokenKind,
) -> Option<&str> {
    if attachment.kind().as_str() != media_kind.as_str() {
        return None;
    }

    extract_upload_response_token(media_kind, attachment.raw_payload())
}

/// Extracts token from an upload ticket.
#[must_use]
pub fn extract_upload_ticket_token(ticket: &UploadTicket) -> Option<&str> {
    ticket.token()
}

/// Extracts token from video metadata response.
#[must_use]
pub fn extract_video_metadata_token(video: &VideoMetadata) -> Option<&str> {
    video.token()
}

/// Extracts media token from raw upload response payload.
///
/// The extractor accepts common MAX response variations:
/// - direct string payload (`"token-value"`),
/// - direct token fields (`token`, `image_token`, `video_token`, ...),
/// - nested object/array containers (`image`, `videos`, `audios`, `files`, ...).
///
/// Whitespace-only token strings are treated as missing.
#[must_use]
pub fn extract_upload_response_token(media_kind: MediaTokenKind, payload: &Value) -> Option<&str> {
    if let Some(token) = non_empty_json_string(payload) {
        return Some(token);
    }

    extract_upload_response_token_inner(media_kind, payload)
        .or_else(|| extract_generic_token(payload))
}

fn extract_upload_response_token_inner(media_kind: MediaTokenKind, value: &Value) -> Option<&str> {
    match value {
        Value::Object(map) => {
            for token_key in media_kind.token_field_candidates() {
                if let Some(token) = map.get(*token_key).and_then(non_empty_json_string) {
                    return Some(token);
                }
            }

            for container_key in media_kind.container_field_candidates() {
                if let Some(container) = map.get(*container_key)
                    && let Some(token) = extract_upload_response_token_inner(media_kind, container)
                {
                    return Some(token);
                }
            }

            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| extract_upload_response_token_inner(media_kind, item)),
        _ => None,
    }
}

fn extract_generic_token(value: &Value) -> Option<&str> {
    match value {
        Value::Object(map) => {
            if let Some(token) = map.get("token").and_then(non_empty_json_string) {
                return Some(token);
            }

            map.values().find_map(extract_generic_token)
        }
        Value::Array(items) => items.iter().find_map(extract_generic_token),
        _ => None,
    }
}

fn non_empty_json_string(value: &Value) -> Option<&str> {
    value
        .as_str()
        .and_then(|candidate| (!candidate.trim().is_empty()).then_some(candidate))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        MediaTokenKind, extract_attachment_token, extract_attachment_token_for_kind,
        extract_upload_response_token, extract_upload_ticket_token, extract_video_metadata_token,
    };
    use crate::models::{
        attachment::{Attachment, KnownAttachmentType},
        upload_ticket::UploadTicket,
        video::VideoMetadata,
    };

    #[test]
    fn extracts_attachment_tokens_for_each_media_type() {
        let cases = [
            (
                MediaTokenKind::Image,
                KnownAttachmentType::Image,
                "image-token",
            ),
            (
                MediaTokenKind::Video,
                KnownAttachmentType::Video,
                "video-token",
            ),
            (
                MediaTokenKind::Audio,
                KnownAttachmentType::Audio,
                "audio-token",
            ),
            (
                MediaTokenKind::File,
                KnownAttachmentType::File,
                "file-token",
            ),
        ];

        for (media_kind, attachment_kind, expected_token) in cases {
            let attachment = Attachment::media_token(attachment_kind, expected_token);

            assert_eq!(extract_attachment_token(&attachment), Some(expected_token));
            assert_eq!(
                extract_attachment_token_for_kind(&attachment, media_kind),
                Some(expected_token)
            );
        }
    }

    #[test]
    fn attachment_token_extraction_returns_none_for_missing_tokens() {
        let cases = [
            (MediaTokenKind::Image, KnownAttachmentType::Image),
            (MediaTokenKind::Video, KnownAttachmentType::Video),
            (MediaTokenKind::Audio, KnownAttachmentType::Audio),
            (MediaTokenKind::File, KnownAttachmentType::File),
        ];

        for (media_kind, attachment_kind) in cases {
            let attachment = Attachment::new(attachment_kind, json!({}));

            assert_eq!(extract_attachment_token(&attachment), None);
            assert_eq!(
                extract_attachment_token_for_kind(&attachment, media_kind),
                None
            );
        }
    }

    #[test]
    fn typed_attachment_extraction_rejects_mismatched_kind() {
        let attachment = Attachment::media_token(KnownAttachmentType::Image, "image-token");
        assert_eq!(
            extract_attachment_token_for_kind(&attachment, MediaTokenKind::Video),
            None
        );
    }

    #[test]
    fn extracts_upload_response_tokens_for_each_media_type() {
        let image_payload = json!({ "image": { "token": "image-token" } });
        let video_payload = json!({ "video_token": "video-token" });
        let audio_payload = json!({ "audios": [{ "token": "audio-token" }] });
        let file_payload = json!({ "token": "file-token" });

        assert_eq!(
            extract_upload_response_token(MediaTokenKind::Image, &image_payload),
            Some("image-token")
        );
        assert_eq!(
            extract_upload_response_token(MediaTokenKind::Video, &video_payload),
            Some("video-token")
        );
        assert_eq!(
            extract_upload_response_token(MediaTokenKind::Audio, &audio_payload),
            Some("audio-token")
        );
        assert_eq!(
            extract_upload_response_token(MediaTokenKind::File, &file_payload),
            Some("file-token")
        );
    }

    #[test]
    fn upload_response_extraction_returns_none_for_missing_tokens() {
        let cases = [
            (MediaTokenKind::Image, json!({ "image": { "url": "x" } })),
            (MediaTokenKind::Video, json!({ "videos": [{ "url": "x" }] })),
            (
                MediaTokenKind::Audio,
                json!({ "audio": { "duration": 10 } }),
            ),
            (MediaTokenKind::File, json!({ "file": { "size": 123 } })),
        ];

        for (media_kind, payload) in cases {
            assert_eq!(extract_upload_response_token(media_kind, &payload), None);
        }
    }

    #[test]
    fn integrates_with_upload_ticket_and_video_models() {
        let ticket: UploadTicket = serde_json::from_value(json!({
            "url": "https://upload.max.ru/ticket",
            "token": "ticket-token"
        }))
        .expect("upload ticket should decode");
        assert_eq!(extract_upload_ticket_token(&ticket), Some("ticket-token"));

        let video: VideoMetadata =
            serde_json::from_value(json!({ "token": "video-metadata-token" }))
                .expect("video metadata should decode");
        assert_eq!(
            extract_video_metadata_token(&video),
            Some("video-metadata-token")
        );
    }

    #[test]
    fn treats_whitespace_only_tokens_as_missing() {
        let payload = json!({ "token": "   " });
        assert_eq!(
            extract_upload_response_token(MediaTokenKind::File, &payload),
            None
        );
    }
}
