//! Video metadata model for `/videos/{videoToken}`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Video metadata returned by MAX API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VideoMetadata {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    duration: Option<u64>,
    #[serde(default, flatten)]
    extra: Map<String, Value>,
}

impl VideoMetadata {
    /// Returns video token if present.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns video URL if present.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        self.url
            .as_deref()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
    }

    /// Returns width in pixels.
    #[must_use]
    pub fn width(&self) -> Option<u32> {
        self.width
    }

    /// Returns height in pixels.
    #[must_use]
    pub fn height(&self) -> Option<u32> {
        self.height
    }

    /// Returns duration in milliseconds when available.
    #[must_use]
    pub fn duration(&self) -> Option<u64> {
        self.duration
    }

    /// Returns forward-compatible extra fields.
    #[must_use]
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}
