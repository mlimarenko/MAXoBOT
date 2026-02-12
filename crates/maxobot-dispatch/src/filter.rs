//! Filter contracts and typed update extraction helpers.

use std::sync::Arc;

use maxobot_core::updates::update_envelope::{
    KnownUpdateType, UpdateEnvelope, UpdateSource, UpdateType,
};
use serde_json::Value;

use crate::handler::DispatchContext;

/// Shared trait object for registered filters.
pub type SharedUpdateFilter = Arc<dyn UpdateFilter>;

/// Predicate contract evaluated before handler execution.
pub trait UpdateFilter: Send + Sync {
    /// Returns `true` when handler execution is allowed.
    fn matches(&self, update: &UpdateEnvelope, context: &DispatchContext) -> bool;
}

impl<F> UpdateFilter for F
where
    F: Fn(&UpdateEnvelope, &DispatchContext) -> bool + Send + Sync,
{
    fn matches(&self, update: &UpdateEnvelope, context: &DispatchContext) -> bool {
        (self)(update, context)
    }
}

/// Composite filter requiring all nested filters to pass.
#[derive(Default, Clone)]
pub struct AllFilter {
    filters: Vec<SharedUpdateFilter>,
}

impl AllFilter {
    /// Creates empty composite.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one filter.
    #[must_use]
    pub fn and(mut self, filter: SharedUpdateFilter) -> Self {
        self.filters.push(filter);
        self
    }
}

impl UpdateFilter for AllFilter {
    fn matches(&self, update: &UpdateEnvelope, context: &DispatchContext) -> bool {
        self.filters
            .iter()
            .all(|filter| filter.matches(update, context))
    }
}

impl std::fmt::Debug for AllFilter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AllFilter")
            .field("filter_count", &self.filters.len())
            .finish()
    }
}

/// Composite filter requiring at least one nested filter to pass.
#[derive(Default, Clone)]
pub struct AnyFilter {
    filters: Vec<SharedUpdateFilter>,
}

impl AnyFilter {
    /// Creates empty composite.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one filter.
    #[must_use]
    pub fn or(mut self, filter: SharedUpdateFilter) -> Self {
        self.filters.push(filter);
        self
    }
}

impl UpdateFilter for AnyFilter {
    fn matches(&self, update: &UpdateEnvelope, context: &DispatchContext) -> bool {
        self.filters
            .iter()
            .any(|filter| filter.matches(update, context))
    }
}

impl std::fmt::Debug for AnyFilter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AnyFilter")
            .field("filter_count", &self.filters.len())
            .finish()
    }
}

/// Negates one nested filter.
#[derive(Clone)]
pub struct NotFilter {
    filter: SharedUpdateFilter,
}

impl NotFilter {
    /// Creates negation filter.
    #[must_use]
    pub fn new(filter: SharedUpdateFilter) -> Self {
        Self { filter }
    }
}

impl UpdateFilter for NotFilter {
    fn matches(&self, update: &UpdateEnvelope, context: &DispatchContext) -> bool {
        !self.filter.matches(update, context)
    }
}

impl std::fmt::Debug for NotFilter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NotFilter")
            .field("has_filter", &true)
            .finish()
    }
}

/// Creates filter for one known update type.
#[must_use]
pub fn by_known_type(expected: KnownUpdateType) -> SharedUpdateFilter {
    Arc::new(
        move |update: &UpdateEnvelope, _context: &DispatchContext| match &update.update_type {
            UpdateType::Known(value) => value == &expected,
            UpdateType::Unknown(_) => false,
        },
    )
}

/// Creates filter for one update source.
#[must_use]
pub fn by_source(expected: UpdateSource) -> SharedUpdateFilter {
    Arc::new(move |update: &UpdateEnvelope, _context: &DispatchContext| update.source == expected)
}

/// Creates filter matching only unknown update types.
#[must_use]
pub fn unknown_only() -> SharedUpdateFilter {
    Arc::new(|update: &UpdateEnvelope, _context: &DispatchContext| {
        matches!(update.update_type, UpdateType::Unknown(_))
    })
}

/// Returns known update type when available.
#[must_use]
pub fn extract_known_type(update: &UpdateEnvelope) -> Option<KnownUpdateType> {
    match &update.update_type {
        UpdateType::Known(value) => Some(value.clone()),
        UpdateType::Unknown(_) => None,
    }
}

/// Returns unknown raw update type when available.
#[must_use]
pub fn extract_unknown_type(update: &UpdateEnvelope) -> Option<&str> {
    match &update.update_type {
        UpdateType::Known(_) => None,
        UpdateType::Unknown(value) => Some(value.as_str()),
    }
}

/// Returns top-level payload object field.
#[must_use]
pub fn extract_payload_field<'a>(update: &'a UpdateEnvelope, field: &str) -> Option<&'a Value> {
    update.payload.get(field)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        AllFilter, AnyFilter, NotFilter, UpdateFilter, by_known_type, by_source,
        extract_known_type, extract_payload_field, extract_unknown_type, unknown_only,
    };
    use crate::handler::DispatchContext;
    use maxobot_core::updates::update_envelope::{
        KnownUpdateType, UpdateEnvelope, UpdateSource, UpdateType,
    };

    fn known_update() -> UpdateEnvelope {
        UpdateEnvelope {
            update_type: UpdateType::Known(KnownUpdateType::MessageCreated),
            timestamp: 1_700_000_000_100_i64,
            payload: json!({"payload": {"chat_id": 10}, "id": "m1"}),
            raw: json!({"update_type": "message_created"}),
            source: UpdateSource::Webhook,
        }
    }

    fn unknown_update() -> UpdateEnvelope {
        UpdateEnvelope {
            update_type: UpdateType::Unknown("future_kind".to_owned()),
            timestamp: 1_700_000_000_200_i64,
            payload: json!({"payload": {"chat_id": 12}}),
            raw: json!({"update_type": "future_kind"}),
            source: UpdateSource::Polling,
        }
    }

    #[test]
    fn composite_filters_apply_expected_logic() {
        let context = DispatchContext::default();
        let update = known_update();
        let all = AllFilter::new()
            .and(by_known_type(KnownUpdateType::MessageCreated))
            .and(by_source(UpdateSource::Webhook));
        let any = AnyFilter::new()
            .or(by_known_type(KnownUpdateType::MessageRemoved))
            .or(by_source(UpdateSource::Webhook));
        let not = NotFilter::new(by_source(UpdateSource::Polling));

        assert!(all.matches(&update, &context));
        assert!(any.matches(&update, &context));
        assert!(not.matches(&update, &context));
    }

    #[test]
    fn unknown_filter_and_extractors_are_forward_compatible() {
        let update = unknown_update();
        let context = DispatchContext::default();

        assert!(unknown_only().matches(&update, &context));
        assert_eq!(extract_unknown_type(&update), Some("future_kind"));
        assert_eq!(extract_known_type(&update), None);
        assert_eq!(
            extract_payload_field(&update, "payload"),
            Some(&json!({"chat_id": 12}))
        );
    }
}
