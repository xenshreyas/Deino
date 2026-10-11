//! The read-only retriever: hybrid with an embedder, keyword-only without
//! one or when it cannot answer, and no writes.

use std::sync::Arc;

use super::*;
use crate::search::hybrid::RRF_K;
use crate::search::semantic::test_harness::Harness;
use crate::search::{HitKind, SearchIndex};

/// A meeting about the graph migration, embedded by the fake model.
async fn indexed() -> Harness {
    let harness = Harness::new().await;
    let meeting = harness
        .meeting(
            "meeting-graph",
            "Platform sync",
            &[
                "we moved the graph database",
                "the migration finished early",
            ],
        )
        .await;
    harness.indexer.index_meeting(&meeting).await.unwrap();
    harness
}

/// Hits as `(kind, has a transcript window)` pairs.
fn shapes(hits: &[Hit]) -> Vec<(HitKind, bool)> {
    hits.iter()
        .map(|hit| (hit.kind, hit.end_ms.is_some()))
        .collect()
}

#[tokio::test]
async fn without_an_embedder_it_answers_by_keyword_alone() {
    let harness = indexed().await;
    let retriever = Retriever::open(harness.database.reader().clone(), None);

    let hits = retriever
        .hybrid_search("graph", &SearchFilters::default(), 5)
        .await
        .unwrap();

    assert!(!retriever.has_embedder());
    assert!(!hits.is_empty());
    assert!(!found_by_both(&hits));
    assert!(shapes(&hits).iter().all(|(_, window)| !window));
}

/// Whether the best hit was found by both lists: reciprocal rank fusion
/// scores a hit one list ranked first at `1 / (RRF_K + 1)`, and one both
/// lists ranked first at twice that.
fn found_by_both(hits: &[Hit]) -> bool {
    let first = 1.0 / (RRF_K + 1.0);
    hits.first()
        .is_some_and(|hit| hit.score > first && hit.score <= 2.0 * first + f64::EPSILON)
}

#[tokio::test]
async fn with_an_embedder_semantic_hits_are_fused_in() {
    let harness = indexed().await;
    let embedder: Arc<dyn Embedder> = harness.embedder.clone();
    let retriever = Retriever::open(harness.database.reader().clone(), Some(embedder));

    let hits = retriever
        .hybrid_search("graph migration", &SearchFilters::default(), 5)
        .await
        .unwrap();

    assert!(retriever.has_embedder());
    assert!(found_by_both(&hits));
}

#[tokio::test]
async fn an_embedder_that_is_not_ready_falls_back_to_keywords() {
    let harness = indexed().await;
    harness.embedder.set_ready(false);
    let embedder: Arc<dyn Embedder> = harness.embedder.clone();
    let retriever = Retriever::open(harness.database.reader().clone(), Some(embedder));

    let hits = retriever
        .hybrid_search("graph", &SearchFilters::default(), 5)
        .await
        .unwrap();

    assert!(!hits.is_empty());
    assert!(shapes(&hits).iter().all(|(_, window)| !window));
}

#[tokio::test]
async fn read_only_adapters_refuse_to_write() {
    let harness = indexed().await;
    let index = SqliteSearchIndex::read_only(harness.database.reader().clone());

    let rebuilt = index.reindex_all().await;

    assert!(matches!(rebuilt, Err(SearchError::ReadOnly)));
}
