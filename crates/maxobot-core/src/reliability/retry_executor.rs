//! Retry orchestration for class-aware transient error handling.

use std::{future::Future, time::Duration};

use tokio::time::sleep;

use crate::{
    diagnostics::request_context::RequestContext,
    errors::api_error::{ApiError, RetryClass as ApiRetryClass},
    reliability::retry_policy::{Jitter, RetryClass, RetryPolicy, RetryPolicyError},
};

/// Telemetry event emitted on retry or terminal give-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryEvent {
    /// Request context snapshot for the failed attempt.
    pub context: RequestContext,
    /// Classified retry class for the failure.
    pub retry_class: RetryClass,
    /// Delay scheduled before the next attempt.
    pub next_delay: Duration,
    /// Redacted error string for diagnostics.
    pub error: String,
}

impl RetryEvent {
    /// Creates a new retry telemetry event.
    #[must_use]
    pub fn new(
        context: RequestContext,
        retry_class: RetryClass,
        next_delay: Duration,
        error: String,
    ) -> Self {
        Self {
            context,
            retry_class,
            next_delay,
            error,
        }
    }
}

/// Callback hooks for observing retry lifecycle events.
pub trait RetryTelemetry: Send + Sync {
    /// Called before sleeping and issuing a retry.
    fn on_retry(&self, _event: &RetryEvent) {}

    /// Called when retry policy decides to stop retrying.
    fn on_give_up(&self, _event: &RetryEvent) {}

    /// Called after operation succeeds.
    fn on_success(&self, _context: &RequestContext) {}
}

/// No-op telemetry implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopRetryTelemetry;

impl RetryTelemetry for NoopRetryTelemetry {}

/// Executes operations under retry policy control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryExecutor {
    policy: RetryPolicy,
}

impl RetryExecutor {
    /// Creates a retry executor after validating retry policy.
    pub fn new(policy: RetryPolicy) -> Result<Self, RetryPolicyError> {
        policy.validate()?;
        Ok(Self { policy })
    }

    /// Returns configured retry policy.
    #[must_use]
    pub fn policy(&self) -> &RetryPolicy {
        &self.policy
    }

    /// Executes an operation with a generated request context and no-op telemetry.
    pub async fn execute<T, F, Fut>(
        &self,
        operation_name: impl Into<String>,
        operation: F,
    ) -> Result<T, ApiError>
    where
        F: FnMut(&RequestContext) -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
    {
        self.execute_with_context(RequestContext::new(operation_name), operation)
            .await
    }

    /// Executes an operation with explicit request context and no-op telemetry.
    pub async fn execute_with_context<T, F, Fut>(
        &self,
        context: RequestContext,
        operation: F,
    ) -> Result<T, ApiError>
    where
        F: FnMut(&RequestContext) -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
    {
        self.execute_with_telemetry(context, operation, &NoopRetryTelemetry)
            .await
    }

    /// Executes an operation with explicit context and telemetry callbacks.
    pub async fn execute_with_telemetry<T, F, Fut, Telemetry>(
        &self,
        mut context: RequestContext,
        mut operation: F,
        telemetry: &Telemetry,
    ) -> Result<T, ApiError>
    where
        F: FnMut(&RequestContext) -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
        Telemetry: RetryTelemetry,
    {
        loop {
            match operation(&context).await {
                Ok(result) => {
                    telemetry.on_success(&context);
                    return Ok(result);
                }
                Err(error) => {
                    let retry_class = classify_error(&error);

                    if !self.policy.should_retry(retry_class, context.attempt) {
                        telemetry.on_give_up(&RetryEvent::new(
                            context.clone(),
                            retry_class,
                            Duration::ZERO,
                            error.redacted_message(),
                        ));
                        return Err(error);
                    }

                    let delay = self.compute_retry_delay(context.attempt, retry_class);
                    telemetry.on_retry(&RetryEvent::new(
                        context.clone(),
                        retry_class,
                        delay,
                        error.redacted_message(),
                    ));
                    sleep(delay).await;
                    context = context.next_attempt();
                }
            }
        }
    }

    fn compute_retry_delay(&self, attempt: u32, retry_class: RetryClass) -> Duration {
        if matches!(retry_class, RetryClass::Immediate) {
            return Duration::ZERO;
        }

        let base_delay = self.policy.backoff.delay_for_retry(attempt);
        apply_jitter(base_delay, &self.policy.jitter, attempt)
    }
}

fn classify_error(error: &ApiError) -> RetryClass {
    if let ApiError::HttpStatus {
        code: Some(code), ..
    } = error
        && code == "attachment.not.ready"
    {
        return RetryClass::AttachmentNotReady;
    }

    if matches!(error, ApiError::Transport(_)) {
        return RetryClass::Transport;
    }

    match error.retry_class() {
        ApiRetryClass::None => RetryClass::None,
        ApiRetryClass::Backoff => RetryClass::Backoff,
        ApiRetryClass::RateLimited => RetryClass::RateLimited,
    }
}

fn apply_jitter(base_delay: Duration, jitter: &Jitter, attempt: u32) -> Duration {
    match jitter {
        Jitter::None => base_delay,
        Jitter::Full { max_jitter } => {
            let max_jitter_ms = max_jitter.as_millis();
            if max_jitter_ms == 0 {
                return base_delay;
            }

            let modulus = max_jitter_ms.saturating_add(1);
            let jitter_ms = u128::from(attempt).saturating_mul(31).saturating_add(17) % modulus;
            let jitter_ms = u64::try_from(jitter_ms).unwrap_or(u64::MAX);
            base_delay.saturating_add(Duration::from_millis(jitter_ms))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };
    use std::time::Duration;

    use http::StatusCode;
    use parking_lot::Mutex;

    use super::{RetryEvent, RetryExecutor, RetryTelemetry};
    use crate::{
        diagnostics::request_context::RequestContext,
        errors::api_error::ApiError,
        reliability::retry_policy::{
            BackoffStrategy, Jitter, RetryClass, RetryClassFilter, RetryPolicy,
        },
    };

    #[derive(Debug, Default)]
    struct CapturingTelemetry {
        retries: Mutex<Vec<RetryEvent>>,
        give_ups: Mutex<Vec<RetryEvent>>,
        successes: Mutex<Vec<RequestContext>>,
    }

    impl RetryTelemetry for CapturingTelemetry {
        fn on_retry(&self, event: &RetryEvent) {
            self.retries.lock().push(event.clone());
        }

        fn on_give_up(&self, event: &RetryEvent) {
            self.give_ups.lock().push(event.clone());
        }

        fn on_success(&self, context: &RequestContext) {
            self.successes.lock().push(context.clone());
        }
    }

    #[tokio::test]
    async fn retries_backoff_error_until_success() {
        let policy = RetryPolicy {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed {
                delay: Duration::from_millis(1),
            },
            jitter: Jitter::None,
            retry_classes: RetryClassFilter::new([RetryClass::Backoff]),
        };
        let executor = RetryExecutor::new(policy).expect("policy should validate");
        let calls = Arc::new(AtomicU32::new(0));

        let result = executor
            .execute("send_message", {
                let calls = Arc::clone(&calls);
                move |_| {
                    let calls = Arc::clone(&calls);
                    async move {
                        let attempt = calls.fetch_add(1, Ordering::SeqCst) + 1;
                        if attempt < 3 {
                            Err(ApiError::from_status(
                                StatusCode::SERVICE_UNAVAILABLE,
                                None,
                                Some("service unavailable".to_owned()),
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

    #[tokio::test]
    async fn gives_up_when_retry_class_not_allowed() {
        let policy = RetryPolicy {
            max_attempts: 5,
            backoff: BackoffStrategy::Fixed {
                delay: Duration::from_millis(1),
            },
            jitter: Jitter::None,
            retry_classes: RetryClassFilter::new([RetryClass::RateLimited]),
        };
        let executor = RetryExecutor::new(policy).expect("policy should validate");
        let calls = Arc::new(AtomicU32::new(0));

        let result: Result<(), ApiError> = executor
            .execute("send_message", {
                let calls = Arc::clone(&calls);
                move |_| {
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(ApiError::from_status(
                            StatusCode::SERVICE_UNAVAILABLE,
                            None,
                            None,
                        ))
                    }
                }
            })
            .await;

        assert!(matches!(result, Err(ApiError::HttpStatus { .. })));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn emits_retry_and_give_up_telemetry() {
        let policy = RetryPolicy {
            max_attempts: 2,
            backoff: BackoffStrategy::Fixed {
                delay: Duration::from_millis(1),
            },
            jitter: Jitter::None,
            retry_classes: RetryClassFilter::new([RetryClass::Backoff]),
        };
        let executor = RetryExecutor::new(policy).expect("policy should validate");
        let telemetry = CapturingTelemetry::default();
        let context = RequestContext::new("send_message");

        let result: Result<(), ApiError> = executor
            .execute_with_telemetry(
                context,
                |_| async {
                    Err(ApiError::from_status(
                        StatusCode::SERVICE_UNAVAILABLE,
                        None,
                        Some("temporary".to_owned()),
                    ))
                },
                &telemetry,
            )
            .await;

        assert!(matches!(result, Err(ApiError::HttpStatus { .. })));
        assert_eq!(telemetry.retries.lock().len(), 1);
        assert_eq!(telemetry.give_ups.lock().len(), 1);
        assert_eq!(telemetry.successes.lock().len(), 0);
        assert_eq!(telemetry.retries.lock()[0].context.attempt, 1);
        assert_eq!(telemetry.give_ups.lock()[0].context.attempt, 2);
    }

    #[test]
    fn classifies_attachment_not_ready() {
        let policy = RetryPolicy {
            retry_classes: RetryClassFilter::new([RetryClass::AttachmentNotReady]),
            ..RetryPolicy::default()
        };
        let executor = RetryExecutor::new(policy).expect("policy should validate");

        let delay = executor.compute_retry_delay(1, RetryClass::AttachmentNotReady);
        assert!(delay >= Duration::from_millis(250));
    }
}
