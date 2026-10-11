//! [`Retriever`]: hybrid search for a process other than the app, such as
//! the `deino` CLI and MCP server (DEV-02/04), which open the database
//! read-only and may run while the app is closed.
//!
//! It needs only a read-only pool and, optionally, an [`Embedder`]. Without
//! one (the embedding model is not downloaded, or no llama-helper runs),
//! every search is keyword-only (FTS5), with the same filters and hits. The
//! same happens per query when the embedder is not ready, fails, or misses
//! [`QUERY_DEADLINE`](super::semantic::QUERY_DEADLINE). Vectors load from the
//! `embeddings` table on the first semantic query (about 120 MB for 40,000
//! chunks), so a long-lived process should keep one `Retriever`.

use std::sync::Arc;

use sqlx::SqlitePool;

use super::semantic::{
    Embedder, NoSemanticSearch, SemanticIndex, SemanticSearch, SharedVectors, SqliteEmbeddingStore,
};
use super::{Hit, SearchError, SearchFilters, SearchService, SqliteSearchIndex};

/// Read-only hybrid search over Deino's database.
pub struct Retriever {
    service: SearchService,
    semantic: bool,
}

impl Retriever {
    /// Search over `pool` (a read-only connection to Deino's database),
    /// fusing in semantic hits when `embedder` is given and ready.
    pub fn open(pool: SqlitePool, embedder: Option<Arc<dyn Embedder>>) -> Self {
        let index = Arc::new(SqliteSearchIndex::read_only(pool.clone()));
        let has_embedder = embedder.is_some();
        let semantic: Arc<dyn SemanticSearch> = match embedder {
            Some(embedder) => {
                let store = Arc::new(SqliteEmbeddingStore::read_only(pool));
                let vectors = Arc::new(SharedVectors::new(store.clone(), embedder.model()));
                Arc::new(SemanticIndex::new(store, embedder, vectors))
            }
            None => Arc::new(NoSemanticSearch),
        };
        Self {
            service: SearchService::new(index.clone(), index, semantic),
            semantic: has_embedder,
        }
    }

    /// Whether an embedder was given; even then a query falls back to
    /// keywords when the model cannot answer in time.
    pub fn has_embedder(&self) -> bool {
        self.semantic
    }

    /// The `limit` best hits for `query` within `filters`, best first:
    /// keyword and semantic fused by reciprocal rank, or keyword alone.
    /// `query` may hold `@mentions` and domains, as in the app's palette;
    /// its words are matched with OR, as a question's.
    pub async fn hybrid_search(
        &self,
        query: &str,
        filters: &SearchFilters,
        limit: usize,
    ) -> Result<Vec<Hit>, SearchError> {
        self.service.search_hybrid(query, filters, limit).await
    }
}

#[cfg(test)]
#[path = "retrieval_tests.rs"]
mod tests;
