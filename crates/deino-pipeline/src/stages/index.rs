//! `index` (order 80, ORG-08 and AI-01): makes a processed meeting
//! searchable through `deino_core::search::SearchIndexing`.
//!
//! The keyword part refreshes the meeting's search documents (the triggers
//! already keep them in step; this repairs any drift). The semantic part
//! embeds the meeting's chunks with the local model; without the model the
//! stage is skipped with a reason, and the Settings backfill embeds the
//! meeting once the model is downloaded.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::search::{MeetingIndexOutcome, SearchIndexing};
use deino_domain::{ErrorClass, MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name; the meeting merge re-runs it (ORG-07).
pub const INDEX_STAGE: &str = "index";
/// The stage's stored name.
const ID: &str = INDEX_STAGE;
/// Where it runs: after notes, tags and folders (PARITY_PLAN §4.2).
const ORDER: u16 = 80;
/// Shown when only keyword search covers the meeting.
const NO_MODEL: &str = "Keyword search is ready. Download the search model in Settings → Search index to also find this meeting by meaning.";

/// Indexes the meeting for search.
pub struct IndexStage {
    indexing: Arc<dyn SearchIndexing>,
}

impl IndexStage {
    /// The stage over the search indexer.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            indexing: Arc::clone(&ports.search),
        }
    }
}

crate::registry::register_stage!(IndexStage);

#[async_trait]
impl PipelineStage for IndexStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let indexed = tokio::select! {
            indexed = self.indexing.index_meeting(&meeting) => indexed,
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
        };
        match indexed {
            Ok(MeetingIndexOutcome::Indexed(_)) => Ok(StageOutcome::Done),
            Ok(MeetingIndexOutcome::KeywordOnly) => Ok(StageOutcome::Skipped(NO_MODEL)),
            Err(error) if error.is_retryable() => Err(StageError::retryable_from(&error)),
            Err(error) => Err(StageError::permanent(error.code(), error.to_string())),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedIndexing;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};

    use async_trait::async_trait;
    use deino_core::search::{MeetingIndexOutcome, SearchError, SearchIndexing};
    use deino_domain::MeetingId;

    /// A [`SearchIndexing`] for tests: records each meeting, indexes nothing.
    #[derive(Debug, Default)]
    pub struct RecordedIndexing {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedIndexing {
        /// Every meeting indexed, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl SearchIndexing for RecordedIndexing {
        async fn index_meeting(
            &self,
            meeting: &MeetingId,
        ) -> Result<MeetingIndexOutcome, SearchError> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(meeting.clone());
            Ok(MeetingIndexOutcome::KeywordOnly)
        }
    }
}

#[cfg(test)]
#[path = "index_tests.rs"]
mod tests;
