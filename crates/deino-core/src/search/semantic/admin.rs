//! Settings → AI → Search index: the embedding model and the backfill.

use std::sync::Arc;

use serde::Serialize;

use super::{Backfill, BackfillProgress, EmbeddingStore};
use crate::models::{ModelFileView, ModelService};
use crate::search::SearchError;

/// What the Search index page shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    /// The embedding model file and whether it is on this Mac.
    pub model: Option<ModelFileView>,
    /// Meetings with something to embed.
    pub total_meetings: u32,
    /// Of those, the ones embedded by the current model.
    pub indexed_meetings: u32,
    /// The backfill's progress.
    pub backfill: BackfillProgress,
}

/// What a [`SemanticAdmin`] manages.
pub struct AdminParts {
    /// The embedding model's files (the catalog's embedding entry).
    pub models: Arc<ModelService>,
    /// The stored chunks, for coverage.
    pub store: Arc<dyn EmbeddingStore>,
    /// The backfill.
    pub backfill: Arc<Backfill>,
    /// The embedding model's name, as stored with its vectors.
    pub model: String,
}

/// Manages the semantic index for Settings.
pub struct SemanticAdmin {
    parts: AdminParts,
}

impl SemanticAdmin {
    /// An admin over its parts.
    pub fn new(parts: AdminParts) -> Self {
        Self { parts }
    }

    /// The model, coverage and backfill now.
    pub async fn status(&self) -> Result<IndexStatus, SearchError> {
        let model = self.parts.models.list().await?.into_iter().next();
        let coverage = self.parts.store.coverage(&self.parts.model).await?;
        Ok(IndexStatus {
            model,
            total_meetings: coverage.total,
            indexed_meetings: coverage.indexed,
            backfill: self.parts.backfill.progress(),
        })
    }

    /// Downloads and verifies the model (resuming a partial download), then
    /// starts the backfill. Returns when the download ends.
    pub async fn download_model(&self) -> Result<IndexStatus, SearchError> {
        let file = self.parts.models.recommended()?;
        self.parts.models.download(&file).await?;
        self.parts.backfill.start();
        self.status().await
    }

    /// Cancels the model's download; false when none was running.
    pub fn cancel_download(&self) -> Result<bool, SearchError> {
        let file = self.parts.models.recommended()?;
        Ok(self.parts.models.cancel(&file))
    }

    /// Deletes the model file. Stored vectors stay, for when it comes back.
    pub async fn delete_model(&self) -> Result<IndexStatus, SearchError> {
        let file = self.parts.models.recommended()?;
        self.parts.models.delete(&file).await?;
        self.status().await
    }

    /// Starts the backfill (a no-op while one runs). Fails when the model is
    /// not downloaded.
    pub async fn start_backfill(&self) -> Result<IndexStatus, SearchError> {
        let file = self.parts.models.recommended()?;
        if self.parts.models.ready_path(&file).await.is_err() {
            return Err(SearchError::ModelMissing);
        }
        self.parts.backfill.start();
        self.status().await
    }
}
