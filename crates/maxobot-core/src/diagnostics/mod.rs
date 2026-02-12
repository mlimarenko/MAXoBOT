//! Diagnostics, telemetry hooks, and redaction helpers.

/// Per-request context metadata for tracing and attempt accounting.
pub mod request_context;

/// Token/secret masking and payload truncation utilities.
pub mod redaction;

/// Structured telemetry events for update lifecycle phases.
pub mod update_telemetry;
