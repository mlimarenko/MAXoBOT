//! Axum adapter for webhook verification + parsing.

use std::{future::Future, sync::Arc};

use axum::{
    Router,
    body::Bytes,
    extract::Request,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use maxobot_core::updates::update_envelope::UpdateEnvelope;
use maxobot_dispatch::{DispatchContext, SequentialDispatcher};
use tracing::warn;

use crate::{
    parser::parse_webhook_payload,
    verifier::{WebhookVerifier, WebhookVerifyError},
};

/// Axum webhook adapter error.
#[derive(Debug, thiserror::Error)]
pub enum AxumWebhookError {
    /// Verification failed.
    #[error(transparent)]
    Verify(#[from] WebhookVerifyError),
    /// Parsing failed.
    #[error(transparent)]
    Parse(#[from] crate::parser::WebhookParseError),
}

/// Builds an axum router for webhook endpoint.
pub fn webhook_router<H, Fut>(verifier: WebhookVerifier, handler: H) -> Router
where
    H: Fn(UpdateEnvelope) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    Router::new().route(
        "/webhook",
        post(move |headers: HeaderMap, body: Bytes| {
            let verifier = verifier.clone();
            let handler = handler.clone();
            async move {
                match verify_and_parse(&verifier, &headers, &body) {
                    Ok(envelope) => match handler(envelope).await {
                        Ok(()) => StatusCode::OK,
                        Err(error) => {
                            warn!("webhook handler failure: {error}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        }
                    },
                    Err(AxumWebhookError::Verify(_)) => StatusCode::UNAUTHORIZED,
                    Err(AxumWebhookError::Parse(_)) => StatusCode::BAD_REQUEST,
                }
            }
        }),
    )
}

/// Builds an axum router and hands parsed updates to sequential dispatcher.
pub fn webhook_router_with_dispatcher(
    verifier: WebhookVerifier,
    dispatcher: Arc<SequentialDispatcher>,
) -> Router {
    webhook_router(verifier, move |update| {
        let dispatcher = Arc::clone(&dispatcher);
        async move {
            dispatcher
                .dispatch_with_context(update, DispatchContext::default())
                .await
                .map(|_| ())
                .map_err(|error| format!("{error}"))
        }
    })
}

/// Parses webhook request from raw headers/body.
pub fn verify_and_parse(
    verifier: &WebhookVerifier,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<UpdateEnvelope, AxumWebhookError> {
    verifier.verify(headers)?;
    parse_webhook_payload(body).map_err(Into::into)
}

/// Extracts raw bytes from request.
pub async fn request_body_bytes(request: Request) -> Result<Bytes, StatusCode> {
    axum::body::to_bytes(request.into_body(), usize::MAX)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)
}
