//! Router registration and selector matching.

use std::sync::Arc;

use maxobot_core::updates::update_envelope::{KnownUpdateType, UpdateEnvelope, UpdateType};

use crate::{
    filter::SharedUpdateFilter,
    handler::{DispatchContext, SharedUpdateHandler},
};

/// Route identifier returned from registration calls.
pub type RouteId = u64;

/// Update selector for routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateSelector {
    /// Match every update type.
    Any,
    /// Match one known update type.
    Known(KnownUpdateType),
    /// Match unknown update types only.
    Unknown,
    /// Match exact raw update type string.
    Raw(String),
}

impl UpdateSelector {
    /// Returns whether selector matches the given update.
    #[must_use]
    pub fn matches(&self, update: &UpdateEnvelope) -> bool {
        match self {
            Self::Any => true,
            Self::Known(expected) => match &update.update_type {
                UpdateType::Known(actual) => actual == expected,
                UpdateType::Unknown(_) => false,
            },
            Self::Unknown => matches!(update.update_type, UpdateType::Unknown(_)),
            Self::Raw(expected) => update.update_type.as_str() == expected,
        }
    }
}

/// One route registration entry.
#[derive(Clone)]
pub struct RouteEntry {
    id: RouteId,
    selector: UpdateSelector,
    priority: i32,
    insertion_order: u64,
    handler: SharedUpdateHandler,
    filters: Vec<SharedUpdateFilter>,
}

impl RouteEntry {
    /// Returns route identifier.
    #[must_use]
    pub const fn id(&self) -> RouteId {
        self.id
    }

    /// Returns route selector.
    #[must_use]
    pub fn selector(&self) -> &UpdateSelector {
        &self.selector
    }

    /// Returns configured route priority.
    #[must_use]
    pub const fn priority(&self) -> i32 {
        self.priority
    }

    /// Returns route handler.
    #[must_use]
    pub fn handler(&self) -> &SharedUpdateHandler {
        &self.handler
    }

    /// Returns route filters.
    #[must_use]
    pub fn filters(&self) -> &[SharedUpdateFilter] {
        &self.filters
    }

    /// Returns whether selector matches update.
    #[must_use]
    pub fn matches_selector(&self, update: &UpdateEnvelope) -> bool {
        self.selector.matches(update)
    }

    /// Returns whether all filters match update.
    #[must_use]
    pub fn passes_filters(&self, update: &UpdateEnvelope, context: &DispatchContext) -> bool {
        self.filters
            .iter()
            .all(|filter| filter.matches(update, context))
    }
}

impl std::fmt::Debug for RouteEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RouteEntry")
            .field("id", &self.id)
            .field("selector", &self.selector)
            .field("priority", &self.priority)
            .field("insertion_order", &self.insertion_order)
            .field("filter_count", &self.filters.len())
            .finish_non_exhaustive()
    }
}

/// Mutable route registry.
#[derive(Debug, Clone, Default)]
pub struct Router {
    routes: Vec<RouteEntry>,
    next_route_id: RouteId,
    next_insertion_order: u64,
}

impl Router {
    /// Creates empty router.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a route with default priority `0`.
    pub fn register(&mut self, selector: UpdateSelector, handler: SharedUpdateHandler) -> RouteId {
        self.register_with_filters(selector, 0, Vec::new(), handler)
    }

    /// Registers a route with explicit priority.
    pub fn register_with_priority(
        &mut self,
        selector: UpdateSelector,
        priority: i32,
        handler: SharedUpdateHandler,
    ) -> RouteId {
        self.register_with_filters(selector, priority, Vec::new(), handler)
    }

    /// Registers a route with explicit filters and priority.
    pub fn register_with_filters(
        &mut self,
        selector: UpdateSelector,
        priority: i32,
        filters: Vec<SharedUpdateFilter>,
        handler: SharedUpdateHandler,
    ) -> RouteId {
        let route_id = self.next_route_id;
        self.next_route_id = self.next_route_id.saturating_add(1);

        let insertion_order = self.next_insertion_order;
        self.next_insertion_order = self.next_insertion_order.saturating_add(1);

        self.routes.push(RouteEntry {
            id: route_id,
            selector,
            priority,
            insertion_order,
            handler,
            filters,
        });

        route_id
    }

    /// Removes route by id.
    pub fn remove(&mut self, route_id: RouteId) -> bool {
        if let Some(index) = self.routes.iter().position(|route| route.id == route_id) {
            self.routes.remove(index);
            true
        } else {
            false
        }
    }

    /// Returns route entries sorted by priority (desc) and insertion order (asc).
    #[must_use]
    pub fn routes_by_priority(&self) -> Vec<&RouteEntry> {
        let mut ordered: Vec<&RouteEntry> = self.routes.iter().collect();
        ordered.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.insertion_order.cmp(&right.insertion_order))
        });
        ordered
    }

    /// Returns matching route entries sorted by priority and insertion order.
    #[must_use]
    pub fn matching_routes(
        &self,
        update: &UpdateEnvelope,
        context: &DispatchContext,
    ) -> Vec<&RouteEntry> {
        self.routes_by_priority()
            .into_iter()
            .filter(|route| route.matches_selector(update) && route.passes_filters(update, context))
            .collect()
    }
}

/// Helper for wrapping any handler implementation into shared object.
#[must_use]
pub fn shared_handler<H>(handler: H) -> SharedUpdateHandler
where
    H: crate::handler::UpdateHandler + 'static,
{
    Arc::new(handler)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Router, UpdateSelector, shared_handler};
    use crate::{filter::by_source, handler::DispatchContext};
    use maxobot_core::updates::update_envelope::{
        KnownUpdateType, UpdateEnvelope, UpdateSource, UpdateType,
    };

    fn fixture_update() -> UpdateEnvelope {
        UpdateEnvelope {
            update_type: UpdateType::Known(KnownUpdateType::MessageCreated),
            timestamp: 1_700_000_000_000_i64,
            payload: json!({"payload": {"chat_id": 1}}),
            raw: json!({"update_type": "message_created"}),
            source: UpdateSource::Webhook,
        }
    }

    #[test]
    fn registration_orders_routes_by_priority_then_insertion() {
        let mut router = Router::new();

        let low = router.register_with_priority(
            UpdateSelector::Any,
            1,
            shared_handler(|_, _| async { Ok(()) }),
        );
        let high = router.register_with_priority(
            UpdateSelector::Any,
            10,
            shared_handler(|_, _| async { Ok(()) }),
        );
        let mid = router.register_with_priority(
            UpdateSelector::Any,
            3,
            shared_handler(|_, _| async { Ok(()) }),
        );

        let ordered = router.routes_by_priority();
        assert_eq!(ordered.len(), 3);
        assert_eq!(ordered[0].id(), high);
        assert_eq!(ordered[1].id(), mid);
        assert_eq!(ordered[2].id(), low);
    }

    #[test]
    fn matching_routes_apply_selector_and_filters() {
        let mut router = Router::new();
        let update = fixture_update();
        let context = DispatchContext::default();

        let should_match = router.register_with_filters(
            UpdateSelector::Known(KnownUpdateType::MessageCreated),
            1,
            vec![by_source(UpdateSource::Webhook)],
            shared_handler(|_, _| async { Ok(()) }),
        );
        let should_not_match = router.register_with_filters(
            UpdateSelector::Unknown,
            2,
            vec![by_source(UpdateSource::Polling)],
            shared_handler(|_, _| async { Ok(()) }),
        );

        let matching = router.matching_routes(&update, &context);
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].id(), should_match);

        assert!(router.remove(should_not_match));
        assert!(!router.remove(999));
    }
}
