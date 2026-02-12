//! Async token-bucket executor for outbound request gating.

use std::{future::Future, sync::Arc, time::Duration};

use parking_lot::Mutex;
use tokio::time::{Instant, sleep};

use crate::{
    client::http_executor::{HttpExecutor, HttpRequest, HttpResponse},
    errors::api_error::ApiError,
    reliability::rate_limit_policy::{RateLimitPolicy, RateLimitPolicyError},
};

/// Telemetry callback hooks for rate-limit waits.
pub trait RateLimitTelemetry: Send + Sync {
    /// Called before sleeping while waiting for token refill.
    fn on_wait(&self, _wait: Duration) {}

    /// Called when token was acquired.
    fn on_acquired(&self, _waited: Duration) {}

    /// Called when acquisition timed out.
    fn on_timeout(&self, _timeout: Duration) {}
}

/// No-op telemetry implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopRateLimitTelemetry;

impl RateLimitTelemetry for NoopRateLimitTelemetry {}

#[derive(Debug, Clone)]
struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

/// Outbound executor with local token-bucket limiter.
#[derive(Debug, Clone)]
pub struct RateLimitedExecutor {
    policy: RateLimitPolicy,
    state: Arc<Mutex<BucketState>>,
}

/// Drop-in HTTP executor wrapper with outbound rate limiting.
#[derive(Debug, Clone)]
pub struct RateLimitedHttpExecutor<E> {
    inner: E,
    limiter: RateLimitedExecutor,
}

impl<E> RateLimitedHttpExecutor<E> {
    /// Wraps HTTP executor with token-bucket limiter.
    #[must_use]
    pub fn new(inner: E, limiter: RateLimitedExecutor) -> Self {
        Self { inner, limiter }
    }

    /// Returns wrapped executor.
    #[must_use]
    pub fn inner(&self) -> &E {
        &self.inner
    }

    /// Returns configured limiter.
    #[must_use]
    pub fn limiter(&self) -> &RateLimitedExecutor {
        &self.limiter
    }
}

#[async_trait::async_trait]
impl<E> HttpExecutor for RateLimitedHttpExecutor<E>
where
    E: HttpExecutor,
{
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ApiError> {
        self.limiter
            .execute(|| async { self.inner.execute(request).await })
            .await
    }
}

impl RateLimitedExecutor {
    /// Creates rate-limited executor from policy.
    pub fn new(policy: RateLimitPolicy) -> Result<Self, RateLimitPolicyError> {
        policy.validate()?;
        Ok(Self {
            state: Arc::new(Mutex::new(BucketState {
                tokens: f64::from(policy.token_bucket.initial_tokens),
                last_refill: Instant::now(),
            })),
            policy,
        })
    }

    /// Returns limiter policy.
    #[must_use]
    pub fn policy(&self) -> &RateLimitPolicy {
        &self.policy
    }

    /// Executes operation after token acquisition using no-op telemetry.
    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T, ApiError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
    {
        self.execute_with_telemetry(operation, &NoopRateLimitTelemetry)
            .await
    }

    /// Executes operation after token acquisition with telemetry callbacks.
    pub async fn execute_with_telemetry<T, F, Fut, Telemetry>(
        &self,
        operation: F,
        telemetry: &Telemetry,
    ) -> Result<T, ApiError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
        Telemetry: RateLimitTelemetry,
    {
        self.acquire_token(telemetry).await?;
        operation().await
    }

    async fn acquire_token<Telemetry>(&self, telemetry: &Telemetry) -> Result<(), ApiError>
    where
        Telemetry: RateLimitTelemetry,
    {
        if !self.policy.enabled {
            return Ok(());
        }

        let started = Instant::now();
        let timeout = self.policy.acquire_timeout;
        let refill_per_second = f64::from(self.policy.token_bucket.refill_per_second);
        let capacity = f64::from(self.policy.token_bucket.capacity);

        loop {
            {
                let mut state = self.state.lock();
                refill_tokens(&mut state, refill_per_second, capacity);

                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    drop(state);
                    telemetry.on_acquired(started.elapsed());
                    return Ok(());
                }
            }

            let elapsed = started.elapsed();
            if elapsed >= timeout {
                telemetry.on_timeout(timeout);
                return Err(ApiError::from_status(
                    http::StatusCode::TOO_MANY_REQUESTS,
                    Some("local.rate_limit.timeout".to_owned()),
                    Some(format!(
                        "local rate limiter failed to acquire token in {} ms",
                        timeout.as_millis()
                    )),
                ));
            }

            let remaining = timeout.saturating_sub(elapsed);
            let wait = next_wait_duration(refill_per_second, remaining);
            telemetry.on_wait(wait);
            sleep(wait).await;
        }
    }
}

fn refill_tokens(state: &mut BucketState, refill_per_second: f64, capacity: f64) {
    let now = Instant::now();
    let elapsed = now.duration_since(state.last_refill);
    state.last_refill = now;

    let refill = elapsed.as_secs_f64() * refill_per_second;
    state.tokens = (state.tokens + refill).min(capacity);
}

fn next_wait_duration(refill_per_second: f64, remaining: Duration) -> Duration {
    let base_wait = if refill_per_second <= 0.0 {
        Duration::from_millis(50)
    } else {
        Duration::from_secs_f64(1.0 / refill_per_second)
    };
    base_wait.min(remaining).max(Duration::from_millis(1))
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use parking_lot::Mutex;

    use super::{RateLimitPolicy, RateLimitTelemetry, RateLimitedExecutor};

    #[derive(Debug, Default)]
    struct ProbeTelemetry {
        waits: Mutex<Vec<Duration>>,
        acquired: AtomicUsize,
        timeouts: AtomicUsize,
    }

    impl RateLimitTelemetry for ProbeTelemetry {
        fn on_wait(&self, wait: Duration) {
            self.waits.lock().push(wait);
        }

        fn on_acquired(&self, _waited: Duration) {
            self.acquired.fetch_add(1, Ordering::SeqCst);
        }

        fn on_timeout(&self, _timeout: Duration) {
            self.timeouts.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn executes_operation_when_token_is_available() {
        let policy = RateLimitPolicy::default();
        let executor = RateLimitedExecutor::new(policy).expect("policy should validate");
        let calls = Arc::new(AtomicUsize::new(0));

        let result = executor
            .execute({
                let calls = Arc::clone(&calls);
                move || {
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, crate::errors::api_error::ApiError>(7u32)
                    }
                }
            })
            .await
            .expect("operation should succeed");

        assert_eq!(result, 7u32);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn emits_timeout_when_no_token_can_be_acquired() {
        let mut policy = RateLimitPolicy::default();
        policy.token_bucket.capacity = 1;
        policy.token_bucket.initial_tokens = 0;
        policy.token_bucket.refill_per_second = 1;
        policy.acquire_timeout = Duration::from_millis(2);
        let executor = RateLimitedExecutor::new(policy).expect("policy should validate");
        let telemetry = ProbeTelemetry::default();

        let result = executor
            .execute_with_telemetry(
                || async { Ok::<_, crate::errors::api_error::ApiError>(()) },
                &telemetry,
            )
            .await;

        assert!(matches!(
            result,
            Err(crate::errors::api_error::ApiError::HttpStatus { .. })
        ));
        assert_eq!(telemetry.timeouts.load(Ordering::SeqCst), 1);
    }
}
