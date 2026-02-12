#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(unused_crate_dependencies)]

//! Typed update dispatch runtime for MAXoBOT.
//!
//! This crate provides registration, filtering, middleware, and dispatch
//! orchestration primitives that can run sequentially or with bounded parallelism.

/// Handler trait contracts and execution context.
pub mod handler;

/// Router registration and selector rules.
pub mod router;

/// Predicate-style filter primitives.
pub mod filter;

/// Middleware contracts and invocation chaining.
pub mod middleware;

/// Deterministic sequential dispatch strategy.
pub mod dispatcher_sequential;

/// Bounded-concurrency dispatch strategy.
pub mod dispatcher_parallel;

/// Explicit handling policy for unmatched updates.
pub mod unmatched;

pub use dispatcher_parallel::{
    IndexedDispatchResult, ParallelDispatchOrdering, ParallelDispatcher,
};
pub use dispatcher_sequential::{DispatchOutcome, SequentialDispatcher};
pub use filter::{AllFilter, AnyFilter, NotFilter, SharedUpdateFilter, UpdateFilter};
pub use handler::{
    DispatchContext, DispatchError, DispatchResult, SharedUpdateHandler, UpdateHandler,
    bind_handler_context,
};
pub use middleware::{DispatchMiddleware, MiddlewareChain, SharedDispatchMiddleware};
pub use router::{RouteEntry, RouteId, Router, UpdateSelector};
pub use unmatched::{NoopUnmatchedHandler, SharedUnmatchedHandler, UnmatchedUpdateHandler};
