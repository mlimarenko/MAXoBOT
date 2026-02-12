#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(unused_crate_dependencies)]

//! Core MAXoBOT building blocks.
//!
//! `maxobot-core` owns transport primitives, typed models, reliability policy,
//! update ingestion foundations, and error taxonomy shared across higher layers.

/// Typed API method group clients.
pub mod api;

/// Authentication and credential primitives.
pub mod auth;

/// Fluent helper builders for outbound payload composition.
pub mod builders;

/// HTTP client abstractions and endpoint composition.
pub mod client;

/// Runtime configuration models.
pub mod config;

/// Observability and redaction primitives.
pub mod diagnostics;

/// Explicit error taxonomy for transport, API, and validation failures.
pub mod errors;

/// Typed MAX domain models.
pub mod models;

/// Retry and rate-limit policies plus execution helpers.
pub mod reliability;

/// Test-support helpers used by unit, integration, and contract suites.
pub mod testing;

/// Update parsing, cursoring, and polling primitives.
pub mod updates;

/// Upload lifecycle models and helper components.
pub mod uploads;
