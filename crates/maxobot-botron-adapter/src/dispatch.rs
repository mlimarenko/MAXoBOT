//! Optional dispatch-runtime integration helpers.

use std::future::Future;

use maxobot_core::updates::update_envelope::UpdateEnvelope;
use maxobot_dispatch::{DispatchContext, DispatchError, SequentialDispatcher};

use crate::{
    config::adapter_config::AdapterConfig, context::adapter_context::AdapterContext,
    inbound::map_inbound_update,
};

/// Maps inbound update and dispatches resulting event through provided closure.
pub async fn map_and_dispatch<F, Fut>(
    update: &UpdateEnvelope,
    adapter_context: AdapterContext,
    adapter_config: &AdapterConfig,
    dispatcher: &SequentialDispatcher,
    mut before_dispatch: F,
) -> Result<(), DispatchError>
where
    F: FnMut(&crate::inbound::InboundEvent) -> Fut,
    Fut: Future<Output = Result<(), DispatchError>>,
{
    if let Some(event) = map_inbound_update(update, adapter_context, adapter_config)
        .map_err(|error| DispatchError::Handler(error.to_string()))?
    {
        before_dispatch(&event).await?;
        let envelope = UpdateEnvelope {
            update_type: update.update_type.clone(),
            timestamp: update.timestamp,
            payload: event.payload,
            raw: update.raw.clone(),
            source: update.source,
        };
        dispatcher
            .dispatch_with_context(envelope, DispatchContext::default())
            .await?;
    }

    Ok(())
}
