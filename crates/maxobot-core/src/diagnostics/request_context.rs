//! Request context for tracing and diagnostics.

use std::collections::BTreeMap;

use uuid::Uuid;

/// Per-request diagnostic metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    /// Request trace ID.
    pub trace_id: Uuid,
    /// Logical operation name.
    pub operation: String,
    /// Current attempt number (starts from 1).
    pub attempt: u32,
    /// Sanitized key-value tags.
    pub tags: BTreeMap<String, String>,
}

impl RequestContext {
    /// Creates a new request context for the operation.
    #[must_use]
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::now_v7(),
            operation: sanitize_value(&operation.into(), 96),
            attempt: 1,
            tags: BTreeMap::new(),
        }
    }

    /// Returns a context with an incremented attempt counter.
    #[must_use]
    pub fn next_attempt(mut self) -> Self {
        self.attempt = self.attempt.saturating_add(1);
        self
    }

    /// Adds a sanitized tag entry.
    pub fn insert_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = sanitize_value(&key.into(), 48);
        let value = sanitize_value(&value.into(), 128);
        self.tags.insert(key, value);
    }
}

fn sanitize_value(value: &str, max_len: usize) -> String {
    let compact = value.replace(['\n', '\r', '\t'], " ");
    compact.chars().take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::RequestContext;

    #[test]
    fn sanitizes_tag_values() {
        let mut context = RequestContext::new("operation");
        context.insert_tag("line\nbreak", "value\twith\tnoise");

        assert!(context.tags.contains_key("line break"));
        assert_eq!(context.tags["line break"], "value with noise");
    }
}
