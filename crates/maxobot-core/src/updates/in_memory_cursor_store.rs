//! In-memory cursor store reference implementation.

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::updates::cursor_store::{CursorStore, CursorStoreError};

#[derive(Debug, Default)]
struct CursorState {
    committed: Option<i64>,
    pending: Option<i64>,
}

/// Thread-safe in-memory cursor store.
#[derive(Debug, Default)]
pub struct InMemoryCursorStore {
    state: Mutex<CursorState>,
}

impl InMemoryCursorStore {
    /// Creates empty cursor store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CursorStore for InMemoryCursorStore {
    async fn get_marker(&self) -> Result<Option<i64>, CursorStoreError> {
        Ok(self.state.lock().committed)
    }

    async fn set_marker(&self, marker: Option<i64>) -> Result<(), CursorStoreError> {
        self.state.lock().pending = marker;
        Ok(())
    }

    async fn commit_marker(&self) -> Result<Option<i64>, CursorStoreError> {
        let mut state = self.state.lock();
        if state.pending.is_some() {
            state.committed = state.pending;
        }
        Ok(state.committed)
    }
}

#[cfg(test)]
mod tests {
    use crate::updates::cursor_store::CursorStore;

    use super::InMemoryCursorStore;

    #[tokio::test]
    async fn stores_and_commits_marker() {
        let store = InMemoryCursorStore::new();
        assert_eq!(store.get_marker().await.expect("marker"), None);

        store.set_marker(Some(10)).await.expect("set marker");
        assert_eq!(store.get_marker().await.expect("marker"), None);
        assert_eq!(store.commit_marker().await.expect("commit"), Some(10));
        assert_eq!(store.get_marker().await.expect("marker"), Some(10));
    }
}
