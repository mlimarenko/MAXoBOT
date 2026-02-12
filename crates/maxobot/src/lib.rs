#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(unused_crate_dependencies)]

//! Public MAXoBOT facade for community consumers.
//!
//! This crate is the stable entry point for the SDK and aggregates optional
//! capabilities such as dispatching and webhook adapters.

/// High-level client API entry points and convenience constructors.
pub mod client;

/// Shared imports for consumers that prefer a single-use prelude.
pub mod prelude {
    pub use crate::client::{GenericBotApiClient, ReqwestBotClient, new_reqwest_bot_client};
    pub use maxobot_core::{
        api::uploads::UploadType,
        auth::credentials::BotCredentials,
        builders::message_builder::MessageBuilder,
        config::client_config::ClientConfig,
        errors::api_error::ApiError,
        models::new_message_body::{MessageTextFormat, NewMessageBody},
    };
}

/// Publicly documented API module grouping.
pub mod api {
    pub use maxobot_core::api::*;
}

/// Feature-gated integration namespace for the dispatcher layer.
#[cfg(feature = "dispatch")]
pub mod dispatch {
    pub use maxobot_dispatch::*;
}

/// Feature-gated integration namespace for webhook helpers.
#[cfg(feature = "webhook")]
pub mod webhook {
    pub use maxobot_webhook::*;
}
