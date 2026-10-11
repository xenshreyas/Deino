//! The in-memory index shared by the indexer and the search: loaded from
//! the store the first time a search needs it, then kept in step a meeting
//! at a time by the indexer.

use std::sync::Arc;

use deino_domain::MeetingId;
use tokio::sync::RwLock;

use super::{EmbeddingStore, StoredVector, VectorIndex};
use crate::search::SearchError;

/// The vectors of one embedding model, loaded lazily.
pub struct SharedVectors {
    store: Arc<dyn EmbeddingStore>,
    model: String,
    index: RwLock<Option<VectorIndex>>,
}

impl SharedVectors {
    /// Vectors of `model` from `store`, not loaded yet.
    pub fn new(store: Arc<dyn EmbeddingStore>, model: impl Into<String>) -> Self {
        Self {
            store,
            model: model.into(),
            index: RwLock::new(None),
        }
    }

    /// Runs `read` over the index, loading it first if this is the first use.
    pub async fn read<R>(&self, read: impl FnOnce(&VectorIndex) -> R) -> Result<R, SearchError> {
        {
            let loaded = self.index.read().await;
            if let Some(index) = loaded.as_ref() {
                return Ok(read(index));
            }
        }
        let mut slot = self.index.write().await;
        if slot.is_none() {
            let vectors = self.store.load(&self.model).await?;
            log::info!(target: "search", "loaded {} vectors into the semantic index", vectors.len());
            *slot = Some(VectorIndex::new(vectors));
        }
        Ok(read(slot.get_or_insert_with(VectorIndex::default)))
    }

    /// Replaces `meeting`'s vectors, if the index is loaded (otherwise the
    /// first load reads them from the store anyway).
    pub async fn replace_meeting(&self, meeting: &MeetingId, vectors: Vec<StoredVector>) {
        if let Some(index) = self.index.write().await.as_mut() {
            index.replace_meeting(meeting, vectors);
        }
    }
}
