//! Bounded-concurrency dispatch runtime.

use std::sync::Arc;

use maxobot_core::updates::update_envelope::UpdateEnvelope;
use tokio::{sync::Semaphore, task::JoinSet};

use crate::{
    dispatcher_sequential::{DispatchOutcome, SequentialDispatcher},
    handler::{DispatchContext, DispatchError},
};

/// Ordering strategy for batch dispatch results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelDispatchOrdering {
    /// Return results in the same order as input updates.
    PreserveInput,
    /// Return results in task-completion order.
    AsCompleted,
}

/// Indexed dispatch result for batch processing.
pub type IndexedDispatchResult = (usize, Result<DispatchOutcome, DispatchError>);

/// Parallel dispatcher based on bounded Tokio task concurrency.
#[derive(Debug, Clone)]
pub struct ParallelDispatcher {
    sequential: Arc<SequentialDispatcher>,
    concurrency: usize,
    ordering: ParallelDispatchOrdering,
}

impl ParallelDispatcher {
    /// Creates parallel dispatcher.
    pub fn new(
        sequential: SequentialDispatcher,
        concurrency: usize,
    ) -> Result<Self, DispatchError> {
        if concurrency == 0 {
            return Err(DispatchError::Runtime(
                "parallel dispatcher concurrency must be greater than zero".to_owned(),
            ));
        }

        Ok(Self {
            sequential: Arc::new(sequential),
            concurrency,
            ordering: ParallelDispatchOrdering::PreserveInput,
        })
    }

    /// Sets result ordering strategy.
    #[must_use]
    pub fn with_ordering(mut self, ordering: ParallelDispatchOrdering) -> Self {
        self.ordering = ordering;
        self
    }

    /// Dispatches one update using the wrapped sequential dispatcher.
    pub async fn dispatch(
        &self,
        update: UpdateEnvelope,
        context: DispatchContext,
    ) -> Result<DispatchOutcome, DispatchError> {
        self.sequential.dispatch_with_context(update, context).await
    }

    /// Dispatches a batch of updates with bounded concurrency.
    pub async fn dispatch_batch(
        &self,
        updates: Vec<UpdateEnvelope>,
        base_context: DispatchContext,
    ) -> Vec<IndexedDispatchResult> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut join_set = JoinSet::new();

        for (index, update) in updates.into_iter().enumerate() {
            let semaphore = Arc::clone(&semaphore);
            let dispatcher = Arc::clone(&self.sequential);
            let context = base_context.clone().for_update_index(index);

            join_set.spawn(async move {
                let acquire_result = semaphore.acquire_owned().await;
                match acquire_result {
                    Ok(permit) => {
                        let result = dispatcher.dispatch_with_context(update, context).await;
                        drop(permit);
                        (index, result)
                    }
                    Err(error) => (
                        index,
                        Err(DispatchError::Runtime(format!(
                            "failed to acquire dispatch permit: {error}"
                        ))),
                    ),
                }
            });
        }

        let mut results = Vec::new();
        while let Some(join_result) = join_set.join_next().await {
            match join_result {
                Ok(result) => results.push(result),
                Err(error) => results.push((
                    usize::MAX,
                    Err(DispatchError::Runtime(format!(
                        "parallel dispatch task failed: {error}"
                    ))),
                )),
            }
        }

        if self.ordering == ParallelDispatchOrdering::PreserveInput {
            results.sort_by_key(|(index, _)| *index);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use tokio::time::sleep;

    use super::{ParallelDispatchOrdering, ParallelDispatcher};
    use crate::{
        dispatcher_sequential::SequentialDispatcher,
        handler::DispatchContext,
        router::{Router, UpdateSelector, shared_handler},
    };
    use maxobot_core::updates::update_envelope::{UpdateEnvelope, UpdateSource, UpdateType};

    fn fixture_update(index: usize) -> UpdateEnvelope {
        UpdateEnvelope {
            update_type: UpdateType::Unknown(format!("future_{index}")),
            timestamp: 1_700_000_000_000_i64 + i64::try_from(index).unwrap_or(0),
            payload: json!({"index": index}),
            raw: json!({"update_type": format!("future_{index}")}),
            source: UpdateSource::Polling,
        }
    }

    fn build_dispatcher() -> ParallelDispatcher {
        let mut router = Router::new();
        router.register(
            UpdateSelector::Any,
            shared_handler(
                |update: UpdateEnvelope, context: DispatchContext| async move {
                    let delay = if context.update_index == 0 {
                        Duration::from_millis(20)
                    } else {
                        Duration::from_millis(2)
                    };
                    sleep(delay).await;
                    let _ = update;
                    Ok(())
                },
            ),
        );

        let sequential = SequentialDispatcher::new(router);
        ParallelDispatcher::new(sequential, 2).expect("parallel dispatcher should build")
    }

    #[tokio::test]
    async fn preserve_input_order_returns_results_in_input_order() {
        let dispatcher = build_dispatcher().with_ordering(ParallelDispatchOrdering::PreserveInput);
        let updates = vec![fixture_update(0), fixture_update(1), fixture_update(2)];

        let results = dispatcher
            .dispatch_batch(updates, DispatchContext::default())
            .await;

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 0);
        assert_eq!(results[1].0, 1);
        assert_eq!(results[2].0, 2);
    }

    #[tokio::test]
    async fn as_completed_order_allows_out_of_input_order_results() {
        let dispatcher = build_dispatcher().with_ordering(ParallelDispatchOrdering::AsCompleted);
        let updates = vec![fixture_update(0), fixture_update(1), fixture_update(2)];

        let results = dispatcher
            .dispatch_batch(updates, DispatchContext::default())
            .await;

        assert_eq!(results.len(), 3);
        assert!(
            results.iter().any(|(index, _)| *index == 0),
            "slow first update must be present in output"
        );
    }
}
