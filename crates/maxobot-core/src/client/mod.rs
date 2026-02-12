//! HTTP and endpoint composition primitives for API calls.

/// API route constants and dynamic path composition helpers.
pub mod api_paths;

/// API endpoint resolution and base URL validation.
pub mod endpoint_resolver;

/// HTTP execution abstraction and reqwest adapter.
pub mod http_executor;

/// Normalized request builder with query/body/header helpers.
pub mod request_builder;
