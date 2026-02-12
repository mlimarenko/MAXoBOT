#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(unused_crate_dependencies)]

//! Botron integration boundary for MAXoBOT.
//!
//! This crate remains optional and isolates channel-specific mapping logic from
//! the public MAX SDK primitives.

/// Adapter configuration models.
pub mod config;

/// Adapter runtime context types.
pub mod context;

/// SDK-to-Botron error translation surface.
pub mod errors;

/// Inbound MAX update mapping primitives.
pub mod inbound;

/// Idempotency and correlation key composition helpers.
pub mod idempotency;

/// Outbound Botron action mapping primitives.
pub mod outbound;

/// Optional integrations that depend on dispatch runtime primitives.
#[cfg(feature = "dispatch")]
pub mod dispatch;

pub use config::adapter_config::{AdapterConfig, UnknownEventPolicy};
pub use context::adapter_context::{AdapterContext, ExternalIdentifiers};
pub use errors::translator::{AdapterFailure, BotronFailureClass, translate_api_error};
pub use idempotency::key_composer::KeyComposer;
pub use inbound::{
    InboundEvent, lifecycle_mapper::map_lifecycle_update, map_inbound_update,
    message_mapper::map_message_or_callback, unknown_mapper::map_unknown_update,
};
pub use outbound::action_mapper::{BotronAction, MaxActionExecutor, execute_outbound_action};
