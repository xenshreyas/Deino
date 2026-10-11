//! [`SemanticSearch`] over the in-memory index: embed the query, take the
//! nearest chunks in scope, and read them back as hits.
//!
//! A search never waits long for the model: the query is embedded with a
//! deadline ([`QUERY_DEADLINE`]), and a query that misses it is answered by
//! keywords alone. Recent query vectors are cached, so typing back over a
//! query, or the results page repeating the palette's, costs nothing.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use deino_domain::MeetingId;

use super::{
    ChunkKind, ChunkRow, EmbedRole, Embedder, EmbeddingStore, SemanticSearch, SharedVectors,
};
use crate::search::{Hit, HitKind, SearchError, Snippet, fts, query, snippet};

/// How long a search waits for its query's vector. Embedding a short query
/// takes about 15 ms once the model is loaded; loading it takes longer, and
/// those first searches fall back to keywords.
pub const QUERY_DEADLINE: Duration = Duration::from_millis(300);
/// Query vectors kept; the cache empties when it fills.
const QUERY_CACHE: usize = 64;

/// Semantic search over the `embeddings` of one model.
pub struct SemanticIndex {
    store: Arc<dyn EmbeddingStore>,
    embedder: Arc<dyn Embedder>,
    vectors: Arc<SharedVectors>,
    queries: Mutex<HashMap<String, Arc<Vec<f32>>>>,
    deadline: Duration,
}

impl SemanticIndex {
    /// A search over `vectors`, embedding queries with `embedder` and
    /// reading chunks from `store`.
    pub fn new(
        store: Arc<dyn EmbeddingStore>,
        embedder: Arc<dyn Embedder>,
        vectors: Arc<SharedVectors>,
    ) -> Self {
        Self {
            store,
            embedder,
            vectors,
            queries: Mutex::new(HashMap::new()),
            deadline: QUERY_DEADLINE,
        }
    }

    /// The same search with another query deadline (tests, the eval).
    #[must_use]
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// The query's vector, from the cache or the model within the deadline.
    /// The embedder is cancel-safe, so a missed deadline just drops the call.
    async fn query_vector(&self, query: &str) -> Option<Arc<Vec<f32>>> {
        if let Some(vector) = self.cached().get(query) {
            return Some(Arc::clone(vector));
        }
        let texts = [query.to_owned()];
        let embedding = self.embedder.embed(&texts, EmbedRole::Query);
        let vector = match tokio::time::timeout(self.deadline, embedding).await {
            Ok(Ok(mut vectors)) => Arc::new(vectors.pop()?),
            Ok(Err(error)) => {
                log::warn!(target: "search", "could not embed a search query: {error}");
                return None;
            }
            Err(_) => {
                log::debug!(target: "search", "the query vector missed its deadline");
                return None;
            }
        };
        let mut cache = self.cached();
        if cache.len() >= QUERY_CACHE {
            cache.clear();
        }
        cache.insert(query.to_owned(), Arc::clone(&vector));
        Some(vector)
    }

    fn cached(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<Vec<f32>>>> {
        self.queries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The nearest rows: `None` when the index is empty.
    async fn nearest(
        &self,
        vector: &[f32],
        scope: Option<&[MeetingId]>,
        k: usize,
    ) -> Result<Option<Vec<(i64, f32)>>, SearchError> {
        let allowed: Option<HashSet<&MeetingId>> = scope.map(|meetings| meetings.iter().collect());
        let allow = |meeting: &MeetingId| allowed.as_ref().is_none_or(|set| set.contains(meeting));
        self.vectors
            .read(|index| (!index.is_empty()).then(|| index.search(vector, k, allow)))
            .await
    }
}

#[async_trait]
impl SemanticSearch for SemanticIndex {
    async fn search(
        &self,
        query: &str,
        scope: Option<&[MeetingId]>,
        k: usize,
    ) -> Result<Option<Vec<Hit>>, SearchError> {
        if k == 0 || query.trim().is_empty() || !self.embedder.ready().await {
            return Ok(None);
        }
        let Some(vector) = self.query_vector(query).await else {
            return Ok(None);
        };
        let Some(nearest) = self.nearest(&vector, scope, k).await? else {
            return Ok(None);
        };
        let ids: Vec<i64> = nearest.iter().map(|(id, _)| *id).collect();
        let mut rows: HashMap<i64, ChunkRow> = self
            .store
            .hydrate(&ids)
            .await?
            .into_iter()
            .map(|row| (row.id, row))
            .collect();
        let words = fts::words(&query::parse(query).terms);
        let hits = nearest
            .into_iter()
            .filter_map(|(id, score)| Some(hit(rows.remove(&id)?, f64::from(score), &words)))
            .collect();
        Ok(Some(hits))
    }
}

/// A stored chunk as a hit: transcript windows are moments, sections are
/// notes or summaries; the snippet is the chunk's most relevant line.
fn hit(row: ChunkRow, score: f64, words: &[String]) -> Hit {
    let kind = match row.kind {
        ChunkKind::Transcript => HitKind::Segment,
        ChunkKind::Note => HitKind::Note,
        ChunkKind::Summary => HitKind::Summary,
    };
    Hit {
        kind,
        ref_id: row.ref_id,
        meeting_id: Some(row.meeting_id),
        ts_ms: row.ts_ms,
        end_ms: row.end_ms,
        label: Snippet::plain(row.title),
        snippet: snippet::excerpt(snippet::best_line(&row.text, words), words),
        date: Some(row.date),
        score,
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
