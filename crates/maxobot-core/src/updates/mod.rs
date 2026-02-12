//! Update ingestion, parsing, and cursoring primitives.

/// Marker commit strategy definitions.
pub mod commit_strategy;

/// Cursor persistence trait used by polling runtime.
pub mod cursor_store;

/// In-memory cursor store reference implementation.
pub mod in_memory_cursor_store;

/// Polling vs webhook mode conflict guard.
pub mod mode_guard;

/// Parser utilities for decoding update envelopes and polling pages.
pub mod parser;

/// Polling client with cursor integration.
pub mod polling_client;

/// Polling loop runtime.
pub mod polling_loop;

/// Retry integration for polling fetch/commit flows.
pub mod retry_integration;

/// Typed update envelope and forward-compatible update type model.
pub mod update_envelope;
