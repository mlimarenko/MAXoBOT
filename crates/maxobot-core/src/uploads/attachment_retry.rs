//! Helpers for delayed-media retry handling (`attachment.not.ready`).

use std::future::Future;

use crate::{
    diagnostics::request_context::RequestContext, errors::api_error::ApiError,
    reliability::retry_executor::RetryExecutor,
};

/// Returns `true` if API error corresponds to delayed media readiness.
#[must_use]
pub fn is_attachment_not_ready(error: &ApiError) -> bool {
    matches!(
        error,
        ApiError::HttpStatus {
            code: Some(code), ..
        } if code == "attachment.not.ready"
    )
}

/// Runs operation via retry executor for attachment-readiness scenario.
///
/// The retry policy should allow `AttachmentNotReady` class.
pub async fn retry_attachment_not_ready<T, F, Fut>(
    retry_executor: &RetryExecutor,
    operation_name: impl Into<String>,
    operation: F,
) -> Result<T, ApiError>
where
    F: FnMut(&RequestContext) -> Fut,
    Fut: Future<Output = Result<T, ApiError>>,
{
    retry_executor.execute(operation_name, operation).await
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };
    use std::time::Duration;

    use http::StatusCode;

    use super::{is_attachment_not_ready, retry_attachment_not_ready};
    use crate::{
        errors::api_error::ApiError,
        reliability::{
            retry_executor::RetryExecutor,
            retry_policy::{BackoffStrategy, Jitter, RetryClass, RetryClassFilter, RetryPolicy},
        },
    };

    #[test]
    fn classifies_attachment_not_ready_code() {
        let error = ApiError::from_status(
            StatusCode::BAD_REQUEST,
            Some("attachment.not.ready".to_owned()),
            Some("not ready".to_owned()),
        );
        assert!(is_attachment_not_ready(&error));
    }

    #[tokio::test]
    async fn retries_attachment_not_ready_until_success() {
        let policy = RetryPolicy {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed {
                delay: Duration::from_millis(1),
            },
            jitter: Jitter::None,
            retry_classes: RetryClassFilter::new([RetryClass::AttachmentNotReady]),
        };
        let executor = RetryExecutor::new(policy).expect("policy should validate");
        let calls = Arc::new(AtomicU32::new(0));

        let result = retry_attachment_not_ready(&executor, "upload_ready_check", {
            let calls = Arc::clone(&calls);
            move |_| {
                let calls = Arc::clone(&calls);
                async move {
                    let attempt = calls.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt < 3 {
                        Err(ApiError::from_status(
                            StatusCode::BAD_REQUEST,
                            Some("attachment.not.ready".to_owned()),
                            Some("try later".to_owned()),
                        ))
                    } else {
                        Ok(attempt)
                    }
                }
            }
        })
        .await
        .expect("operation should eventually succeed");

        assert_eq!(result, 3);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }
}
