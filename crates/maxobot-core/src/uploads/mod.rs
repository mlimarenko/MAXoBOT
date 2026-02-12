//! Upload lifecycle and media attachment helper primitives.

/// Attachment-not-ready retry helpers.
pub mod attachment_retry;

/// Binary upload helper for upload-ticket URLs.
pub mod multipart_uploader;

/// Upload lifecycle state machine model.
pub mod upload_session;

/// Media token extraction helpers for attachment payloads.
pub mod token_extractor;
