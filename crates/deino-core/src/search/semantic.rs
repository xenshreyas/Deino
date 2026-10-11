//! Semantic search (AI-01, ADR 0028): meetings cut into chunks, embedded on
//! the Mac by `nomic-embed-text-v1.5`, and found by meaning.
//!
//! - [`chunk`]: transcript windows of about 400 tokens cut at speaker turns
//!   (overlapping by a short turn) and note and summary sections;
//! - [`Embedder`]: text to vectors, over the LLM gateway's local `embed`
//!   route ([`GatewayEmbedder`]), with the model's task prefixes;
//! - [`EmbeddingStore`] and [`MeetingSourceReader`]: the `embeddings` table
//!   and what meetings say ([`SqliteEmbeddingStore`]);
//! - [`VectorIndex`] behind [`SharedVectors`]: exact cosine search in memory,
//!   loaded on first use and updated a meeting at a time;
//! - [`SemanticIndexer`]: (re)embeds a meeting, reusing unchanged chunks;
//!   the `index` pipeline stage and the [`Backfill`] call it;
//! - [`SemanticIndex`]: answers [`SemanticSearch`] for the search service,
//!   which fuses it with keyword results (`hybrid`);
//! - [`SemanticAdmin`]: Settings → AI → Search index (the model's download,
//!   coverage, the backfill).
//!
//! Until the model is downloaded nothing here runs: indexing reports
//! [`SearchError::ModelMissing`] and searching answers `None`, so search
//! stays keyword-only.

mod admin;
mod backfill;
pub mod chunk;
mod embedder;
mod engine;
mod indexer;
mod shared;
mod sqlite_store;
mod store;
#[cfg(test)]
mod test_harness;
mod vectors;

use async_trait::async_trait;
use deino_domain::MeetingId;

pub use admin::{AdminParts, IndexStatus, SemanticAdmin};
pub use backfill::{Backfill, BackfillProgress, IndexEvents};
pub use chunk::{Chunk, ChunkKind, TranscriptLine};
pub use embedder::{
    DOCUMENT_PREFIX, EMBED_BATCH, EmbedRole, Embedder, GatewayEmbedder, NoEmbedder, QUERY_PREFIX,
};
pub use engine::SemanticIndex;
pub use indexer::{IndexedMeeting, IndexerParts, SemanticIndexer, chunks_of};
pub use shared::SharedVectors;
pub use sqlite_store::SqliteEmbeddingStore;
pub use store::{
    ChunkRow, Coverage, EmbeddingStore, ExistingEmbedding, MeetingSourceReader, MeetingSources,
    Replacement,
};
pub use vectors::{StoredVector, VectorIndex};

use super::{Hit, SearchError};

/// Finds chunks by meaning, for the search service and chat (AI-02).
#[async_trait]
pub trait SemanticSearch: Send + Sync {
    /// The `k` chunks most similar to `query`, best first, from the meetings
    /// in `scope` (or every meeting outside the trash). `None` when semantic
    /// search cannot answer right now: no model, nothing indexed yet, or the
    /// query could not be embedded in time. Never fails a search for that.
    async fn search(
        &self,
        query: &str,
        scope: Option<&[MeetingId]>,
        k: usize,
    ) -> Result<Option<Vec<Hit>>, SearchError>;
}

/// No semantic search: keyword results only (tests, and a build without
/// the model).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSemanticSearch;

#[async_trait]
impl SemanticSearch for NoSemanticSearch {
    async fn search(
        &self,
        _query: &str,
        _scope: Option<&[MeetingId]>,
        _k: usize,
    ) -> Result<Option<Vec<Hit>>, SearchError> {
        Ok(None)
    }
}
