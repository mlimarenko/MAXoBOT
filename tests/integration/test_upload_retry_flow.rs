use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use http::StatusCode;

use maxobot_core::{
    diagnostics::request_context::RequestContext,
    errors::api_error::ApiError,
    reliability::{
        retry_executor::{RetryEvent, RetryExecutor, RetryTelemetry},
        retry_policy::{BackoffStrategy, Jitter, RetryClass, RetryClassFilter, RetryPolicy},
    },
};

#[derive(Debug, Default)]
struct CapturingRetryTelemetry {
    retries: Mutex<Vec<RetryEvent>>,
    give_ups: Mutex<Vec<RetryEvent>>,
    successes: Mutex<Vec<RequestContext>>,
}

impl RetryTelemetry for CapturingRetryTelemetry {
    fn on_retry(&self, event: &RetryEvent) {
        self.retries
            .lock()
            .expect("retry telemetry lock should not be poisoned")
            .push(event.clone());
    }

    fn on_give_up(&self, event: &RetryEvent) {
        self.give_ups
            .lock()
            .expect("give-up telemetry lock should not be poisoned")
            .push(event.clone());
    }

    fn on_success(&self, context: &RequestContext) {
        self.successes
            .lock()
            .expect("success telemetry lock should not be poisoned")
            .push(context.clone());
    }
}

#[tokio::test]
async fn upload_retry_flow_retries_attachment_not_ready_then_succeeds() {
    let configured_backoff = Duration::from_millis(1);
    let policy = RetryPolicy {
        max_attempts: 3,
        backoff: BackoffStrategy::Fixed {
            delay: configured_backoff,
        },
        jitter: Jitter::None,
        retry_classes: RetryClassFilter::new([RetryClass::AttachmentNotReady]),
    };
    let executor = RetryExecutor::new(policy).expect("retry policy should validate");
    let telemetry = CapturingRetryTelemetry::default();
    let attempts = Arc::new(AtomicU32::new(0));

    let send_result = executor
        .execute_with_telemetry(
            RequestContext::new("upload_send_flow"),
            {
                let attempts = Arc::clone(&attempts);
                move |_| {
                    let attempts = Arc::clone(&attempts);
                    async move {
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                        if attempt == 1 {
                            Err(ApiError::from_status(
                                StatusCode::SERVICE_UNAVAILABLE,
                                Some("attachment.not.ready".to_owned()),
                                Some("attachment is still processing".to_owned()),
                            ))
                        } else {
                            Ok("sent")
                        }
                    }
                }
            },
            &telemetry,
        )
        .await
        .expect("upload flow should succeed after retry");

    assert_eq!(send_result, "sent");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);

    let retries = telemetry
        .retries
        .lock()
        .expect("retry telemetry lock should not be poisoned");
    assert_eq!(retries.len(), 1);
    assert_eq!(retries[0].context.attempt, 1);
    assert_eq!(retries[0].context.operation, "upload_send_flow");
    assert_eq!(retries[0].retry_class, RetryClass::AttachmentNotReady);
    assert_eq!(retries[0].next_delay, configured_backoff);
    drop(retries);

    let give_ups = telemetry
        .give_ups
        .lock()
        .expect("give-up telemetry lock should not be poisoned");
    assert!(give_ups.is_empty());
    drop(give_ups);

    let (success_count, success_attempt, success_operation) = {
        let successes = telemetry
            .successes
            .lock()
            .expect("success telemetry lock should not be poisoned");
        (
            successes.len(),
            successes[0].attempt,
            successes[0].operation.clone(),
        )
    };
    assert_eq!(success_count, 1);
    assert_eq!(success_attempt, 2);
    assert_eq!(success_operation, "upload_send_flow");
}
