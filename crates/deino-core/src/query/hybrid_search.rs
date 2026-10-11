//! [`HybridSearch`]: the [`Search`] port over the app's own search
//! (ORG-08/AI-01's [`Retriever`]): the FTS5 index the app keeps, fused with
//! the semantic index when an [`Embedder`] answers, keyword-only otherwise.
//!
//! [`EmbedProbe`] counts the query embeddings that succeed, so an outcome
//! says whether meaning was actually searched (the retriever falls back to
//! keywords on its own when the model is missing, slow or failing).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use deino_domain::{LocalZone, Millis};
use sqlx::SqlitePool;

use super::link::{meeting_link, moment_link};
use super::{HitKind, QueryError, Search, SearchHit, SearchMode, SearchOutcome, SearchQuery};
use crate::search::semantic::{EmbedRole, Embedder};
use crate::search::{
    FilterChip, FilterKind, Hit, HitKind as IndexKind, Retriever, SearchError, SearchFilters,
};

/// Search over Deino's index from another process, read-only.
pub struct HybridSearch {
    retriever: Retriever,
    probe: Option<Arc<EmbedProbe>>,
    zone: Arc<dyn LocalZone>,
}

impl HybridSearch {
    /// Search over the read-only `pool`, with semantic hits when `embedder`
    /// is given and answers; times in `zone`.
    pub fn new(
        pool: SqlitePool,
        embedder: Option<Arc<dyn Embedder>>,
        zone: Arc<dyn LocalZone>,
    ) -> Self {
        let probe = embedder.map(|embedder| Arc::new(EmbedProbe::new(embedder)));
        let semantic = probe.clone().map(|probe| probe as Arc<dyn Embedder>);
        Self {
            retriever: Retriever::open(pool, semantic),
            probe,
            zone,
        }
    }

    fn answered(&self) -> u64 {
        self.probe.as_ref().map_or(0, |probe| probe.answered())
    }
}

#[async_trait]
impl Search for HybridSearch {
    async fn search(&self, query: &SearchQuery) -> Result<SearchOutcome, QueryError> {
        if query.text.trim().is_empty() {
            return Ok(SearchOutcome {
                mode: SearchMode::Keyword,
                hits: Vec::new(),
            });
        }
        let before = self.answered();
        let limit = usize::try_from(query.limit()).unwrap_or(usize::MAX);
        let found = self
            .retriever
            .hybrid_search(&query.text, &filters(query), limit)
            .await?;
        let mode = if self.answered() > before {
            SearchMode::Hybrid
        } else {
            SearchMode::Keyword
        };
        let zone = self.zone.as_ref();
        let hits = found.into_iter().map(|hit| search_hit(hit, zone)).collect();
        Ok(SearchOutcome { mode, hits })
    }
}

/// The query's meeting as a filter chip.
fn filters(query: &SearchQuery) -> SearchFilters {
    let mut filters = SearchFilters::default();
    if let Some(meeting) = &query.meeting {
        filters.add(FilterChip {
            kind: FilterKind::Meeting,
            id: meeting.to_string(),
            label: meeting.to_string(),
        });
    }
    filters
}

/// An index hit as the library shows it.
fn search_hit(hit: Hit, zone: &dyn LocalZone) -> SearchHit {
    let link = hit.meeting_id.as_ref().map(|meeting| match hit.ts_ms {
        Some(at) => moment_link(meeting, at),
        None => meeting_link(meeting),
    });
    let snippet = if hit.snippet.text.trim().is_empty() {
        hit.label.text.clone()
    } else {
        hit.snippet.text
    };
    SearchHit {
        kind: kind(hit.kind),
        label: hit.label.text,
        held_at: hit.date.map(|date| zone.local(date)),
        at_ms: hit.ts_ms.map(Millis::get),
        meeting_id: hit.meeting_id,
        snippet,
        link,
        score: hit.score,
    }
}

fn kind(kind: IndexKind) -> HitKind {
    match kind {
        IndexKind::Meeting => HitKind::Meeting,
        IndexKind::Note => HitKind::Notes,
        IndexKind::Summary => HitKind::AiNotes,
        IndexKind::Segment => HitKind::Transcript,
        IndexKind::ActionItem => HitKind::ActionItem,
        IndexKind::Person => HitKind::Person,
        IndexKind::Company => HitKind::Company,
        IndexKind::Folder => HitKind::Folder,
    }
}

/// An [`Embedder`] that counts the search queries it embedded.
pub struct EmbedProbe {
    inner: Arc<dyn Embedder>,
    answered: AtomicU64,
}

impl EmbedProbe {
    /// A probe around `inner`.
    pub fn new(inner: Arc<dyn Embedder>) -> Self {
        Self {
            inner,
            answered: AtomicU64::new(0),
        }
    }

    /// How many search queries were embedded so far.
    pub fn answered(&self) -> u64 {
        self.answered.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Embedder for EmbedProbe {
    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn ready(&self) -> bool {
        self.inner.ready().await
    }

    async fn embed(&self, texts: &[String], role: EmbedRole) -> Result<Vec<Vec<f32>>, SearchError> {
        let vectors = self.inner.embed(texts, role).await?;
        if role == EmbedRole::Query {
            self.answered.fetch_add(1, Ordering::SeqCst);
        }
        Ok(vectors)
    }
}

#[cfg(test)]
#[path = "hybrid_search_tests.rs"]
mod tests;
