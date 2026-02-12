//! Secret redaction helpers for diagnostics payloads.

use serde_json::Value;

/// Redaction behavior configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionConfig {
    /// Replacement mask for sensitive values.
    pub mask: String,
    /// Maximum string length for non-sensitive payload fragments.
    pub max_string_len: usize,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            mask: "***".to_owned(),
            max_string_len: 256,
        }
    }
}

/// Redacts sensitive fragments in plain text.
#[must_use]
pub fn redact_text(input: &str, config: &RedactionConfig) -> String {
    let mut output = input.to_owned();
    for marker in [
        "authorization:",
        "authorization=",
        "token=",
        "access_token=",
        "secret=",
        "webhook_secret=",
    ] {
        redact_marker_values(&mut output, marker, &config.mask);
    }

    truncate_string(&output, config.max_string_len)
}

/// Redacts sensitive fields in JSON payload recursively.
#[must_use]
pub fn redact_json(value: &Value, config: &RedactionConfig) -> Value {
    match value {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, value) in map {
                if is_sensitive_key(key) {
                    redacted.insert(key.clone(), Value::String(config.mask.clone()));
                } else {
                    redacted.insert(key.clone(), redact_json(value, config));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|value| redact_json(value, config))
                .collect(),
        ),
        Value::String(text) => Value::String(truncate_string(text, config.max_string_len)),
        Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("token")
        || key.contains("secret")
        || key.contains("authorization")
        || key.contains("password")
}

fn redact_marker_values(output: &mut String, marker: &str, mask: &str) {
    let mut search_from = 0usize;
    loop {
        let haystack = output.to_ascii_lowercase();
        let Some(found_offset) = haystack[search_from..].find(marker) else {
            break;
        };
        let marker_start = search_from + found_offset;
        let mut value_start = marker_start + marker.len();

        while let Some(byte) = output.as_bytes().get(value_start) {
            if *byte == b' ' || *byte == b'"' || *byte == b'\'' {
                value_start += 1;
            } else {
                break;
            }
        }

        let value_end = output[value_start..]
            .find(['&', ' ', '\n', '\r', '"', '\''])
            .map_or(output.len(), |offset| value_start + offset);

        if value_end <= value_start {
            search_from = marker_start + marker.len();
            continue;
        }

        output.replace_range(value_start..value_end, mask);
        search_from = value_start + mask.len();
    }
}

fn truncate_string(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_owned();
    }

    value.chars().take(max_len).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{RedactionConfig, redact_json, redact_text};

    #[test]
    fn redact_text_masks_auth_markers() {
        let config = RedactionConfig::default();
        let input = "Authorization: token123 access_token=abc";
        let redacted = redact_text(input, &config);
        assert!(redacted.contains("Authorization:"));
        assert!(!redacted.contains("token123"));
        assert!(redacted.contains("access_token=***"));
    }

    #[test]
    fn redact_json_masks_sensitive_keys_and_truncates_strings() {
        let config = RedactionConfig {
            mask: "<redacted>".to_owned(),
            max_string_len: 5,
        };
        let payload = json!({
            "token": "abcdef",
            "nested": {
                "webhook_secret": "abcdef",
                "note": "1234567890"
            },
            "array": [{"authorization": "abc"}, "long-value-123"]
        });
        let redacted = redact_json(&payload, &config);

        assert_eq!(redacted["token"], json!("<redacted>"));
        assert_eq!(redacted["nested"]["webhook_secret"], json!("<redacted>"));
        assert_eq!(redacted["nested"]["note"], json!("12345…"));
        assert_eq!(redacted["array"][0]["authorization"], json!("<redacted>"));
        assert_eq!(redacted["array"][1], json!("long-…"));
    }
}
