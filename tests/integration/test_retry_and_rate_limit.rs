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

fn build_executor(
    max_attempts: u32,
    backoff: BackoffStrategy,
    retry_classes: impl IntoIterator<Item = RetryClass>,
) -> RetryExecutor {
    let policy = RetryPolicy {
        max_attempts,
        backoff,
        jitter: Jitter::None,
        retry_classes: RetryClassFilter::new(retry_classes),
    };

    RetryExecutor::new(policy).expect("retry policy should validate")
}

fn captured_retries(telemetry: &CapturingRetryTelemetry) -> Vec<RetryEvent> {
    telemetry
        .retries
        .lock()
        .expect("retry telemetry lock should not be poisoned")
        .clone()
}

fn captured_give_ups(telemetry: &CapturingRetryTelemetry) -> Vec<RetryEvent> {
    telemetry
        .give_ups
        .lock()
        .expect("give-up telemetry lock should not be poisoned")
        .clone()
}

fn success_count(telemetry: &CapturingRetryTelemetry) -> usize {
    telemetry
        .successes
        .lock()
        .expect("success telemetry lock should not be poisoned")
        .len()
}

fn assert_event(
    event: &RetryEvent,
    operation: &str,
    attempt: u32,
    retry_class: RetryClass,
    next_delay: Duration,
) {
    assert_eq!(event.context.operation, operation);
    assert_eq!(event.context.attempt, attempt);
    assert_eq!(event.retry_class, retry_class);
    assert_eq!(event.next_delay, next_delay);
}

fn assert_terminal_status(error: ApiError, expected_status: StatusCode) {
    let ApiError::HttpStatus { status, .. } = error else {
        panic!("expected HTTP status error");
    };
    assert_eq!(status, expected_status);
}

#[tokio::test]
async fn retry_flow_429_rate_limited_uses_rate_limited_class_and_terminal_error() {
    let configured_backoff = Duration::from_millis(2);
    let executor = build_executor(
        2,
        BackoffStrategy::Fixed {
            delay: configured_backoff,
        },
        [RetryClass::RateLimited],
    );
    let telemetry = CapturingRetryTelemetry::default();
    let attempts = Arc::new(AtomicU32::new(0));

    let result: Result<(), ApiError> = executor
        .execute_with_telemetry(
            RequestContext::new("rate_limit_send_flow"),
            {
                let attempts = Arc::clone(&attempts);
                move |_| {
                    let attempts = Arc::clone(&attempts);
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Err(ApiError::from_status(
                            StatusCode::TOO_MANY_REQUESTS,
                            Some("rate.limited".to_owned()),
                            Some("too many requests".to_owned()),
                        ))
                    }
                }
            },
            &telemetry,
        )
        .await;

    let terminal_error = result.expect_err("429 flow should give up after max attempts");
    assert_terminal_status(terminal_error, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);

    let retries = captured_retries(&telemetry);
    assert_eq!(retries.len(), 1);
    assert_event(
        &retries[0],
        "rate_limit_send_flow",
        1,
        RetryClass::RateLimited,
        configured_backoff,
    );

    let give_ups = captured_give_ups(&telemetry);
    assert_eq!(give_ups.len(), 1);
    assert_event(
        &give_ups[0],
        "rate_limit_send_flow",
        2,
        RetryClass::RateLimited,
        Duration::ZERO,
    );

    assert_eq!(success_count(&telemetry), 0);
}

#[tokio::test]
async fn retry_flow_503_service_unavailable_uses_backoff_class_backoff_and_terminal_error() {
    let first_backoff = Duration::from_millis(3);
    let second_backoff = Duration::from_millis(9);
    let executor = build_executor(
        3,
        BackoffStrategy::Exponential {
            initial_delay: first_backoff,
            multiplier: 3,
            max_delay: Duration::from_millis(25),
        },
        [RetryClass::Backoff],
    );
    let telemetry = CapturingRetryTelemetry::default();
    let attempts = Arc::new(AtomicU32::new(0));

    let result: Result<(), ApiError> = executor
        .execute_with_telemetry(
            RequestContext::new("service_unavailable_send_flow"),
            {
                let attempts = Arc::clone(&attempts);
                move |_| {
                    let attempts = Arc::clone(&attempts);
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Err(ApiError::from_status(
                            StatusCode::SERVICE_UNAVAILABLE,
                            Some("temporary.unavailable".to_owned()),
                            Some("service unavailable".to_owned()),
                        ))
                    }
                }
            },
            &telemetry,
        )
        .await;

    let terminal_error = result.expect_err("503 flow should give up after max attempts");
    assert_terminal_status(terminal_error, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(attempts.load(Ordering::SeqCst), 3);

    let retries = captured_retries(&telemetry);
    assert_eq!(retries.len(), 2);
    assert_event(
        &retries[0],
        "service_unavailable_send_flow",
        1,
        RetryClass::Backoff,
        first_backoff,
    );
    assert_event(
        &retries[1],
        "service_unavailable_send_flow",
        2,
        RetryClass::Backoff,
        second_backoff,
    );

    let give_ups = captured_give_ups(&telemetry);
    assert_eq!(give_ups.len(), 1);
    assert_event(
        &give_ups[0],
        "service_unavailable_send_flow",
        3,
        RetryClass::Backoff,
        Duration::ZERO,
    );

    assert_eq!(success_count(&telemetry), 0);
}
