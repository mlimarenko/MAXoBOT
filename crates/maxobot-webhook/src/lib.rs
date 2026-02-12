#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(unused_crate_dependencies)]

//! Webhook verification and parsing surface for MAXoBOT.
//!
//! The crate contains framework-agnostic core components with optional
//! adapters for specific Rust web frameworks.

/// Webhook request verification primitives.
pub mod verifier;

/// Webhook payload parsing primitives.
pub mod parser;

/// Axum integration adapter for webhook request handling.
#[cfg(feature = "axum")]
pub mod axum_adapter;
