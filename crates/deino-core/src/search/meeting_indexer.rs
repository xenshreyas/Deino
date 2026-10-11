//! Indexing one meeting for search: what the `index` pipeline stage does.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::MeetingId;

use super::semantic::{IndexedMeeting, SemanticIndexer};
use super::{SearchError, SearchIndex};

/// How indexing a meeting ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetingIndexOutcome {
    /// Keyword documents refreshed and chunks embedded.
    Indexed(IndexedMeeting),
    /// Keyword documents refreshed; the embedding model is not downloaded,
    /// so the meeting is not in the semantic index yet.
    KeywordOnly,
}

/// Indexes one meeting for search (the `index` pipeline stage's port).
#[async_trait]
pub trait SearchIndexing: Send + Sync {
    /// Indexes `meeting`. Idempotent: running it twice changes nothing.
    /// Dropping the future leaves the indexes consistent (every write is one
    /// transaction).
    async fn index_meeting(&self, meeting: &MeetingId) -> Result<MeetingIndexOutcome, SearchError>;
}

/// Refreshes a meeting's keyword documents and embeds its chunks.
pub struct MeetingIndexer {
    keyword: Arc<dyn SearchIndex>,
    semantic: Arc<SemanticIndexer>,
}

impl MeetingIndexer {
    /// An indexer over the keyword index and the semantic indexer.
    pub fn new(keyword: Arc<dyn SearchIndex>, semantic: Arc<SemanticIndexer>) -> Self {
        Self { keyword, semantic }
    }
}

#[async_trait]
impl SearchIndexing for MeetingIndexer {
    async fn index_meeting(&self, meeting: &MeetingId) -> Result<MeetingIndexOutcome, SearchError> {
        self.keyword.refresh_meeting(meeting).await?;
        match self.semantic.index_meeting(meeting).await {
            Ok(indexed) => Ok(MeetingIndexOutcome::Indexed(indexed)),
            Err(SearchError::ModelMissing) => Ok(MeetingIndexOutcome::KeywordOnly),
            Err(error) => Err(error),
        }
    }
}
