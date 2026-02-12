//! Fixture loader for contract and compatibility tests.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::errors::api_error::ApiError;

/// Loads raw JSON fixture from a path.
pub fn load_json_fixture(path: impl AsRef<Path>) -> Result<Value, ApiError> {
    let path = path.as_ref().to_path_buf();
    let bytes = std::fs::read(&path).map_err(|source| ApiError::FixtureIo {
        path: path.clone(),
        source,
    })?;

    serde_json::from_slice(&bytes).map_err(|source| ApiError::FixtureParse { path, source })
}

/// Loads a fixture and enforces object-shaped JSON schema.
pub fn load_object_fixture(path: impl AsRef<Path>) -> Result<Value, ApiError> {
    let path = path.as_ref().to_path_buf();
    let value = load_json_fixture(&path)?;

    if value.is_object() {
        Ok(value)
    } else {
        Err(ApiError::FixtureSchema {
            path,
            reason: "fixture must be a JSON object".to_owned(),
        })
    }
}

/// Loads a fixture located under fixture root.
pub fn load_fixture_from_root(
    root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<Value, ApiError> {
    let mut absolute = PathBuf::from(root.as_ref());
    absolute.push(relative_path.as_ref());
    load_json_fixture(absolute)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::load_object_fixture;

    #[test]
    fn fails_when_fixture_is_not_object() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();

        let path = std::env::temp_dir().join(format!("maxobot-core-fixture-{nonce}.json"));
        fs::write(&path, "[1,2,3]").expect("write fixture");

        let result = load_object_fixture(&path);
        assert!(result.is_err());
        let _ = fs::remove_file(path);
    }
}
