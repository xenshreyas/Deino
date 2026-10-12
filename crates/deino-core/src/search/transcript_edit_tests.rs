//! TRN-02's acceptance for search: a deleted line, and words an edit
//! replaced, are not found by keyword or by meaning, through the
//! [`Retriever`] the `deino` CLI and MCP server read with, neither the
//! moment the edit commits (the triggers) nor after the re-index; and the
//! re-index brings the new words into the semantic index.

use std::sync::Arc;

use deino_domain::{FakeClock, MeetingId, SegmentId};
use deino_testkit::at;

use super::semantic::test_harness::Harness;
use super::*;
use crate::segments::{Page, SegmentReader, SqliteSegmentRepo};
use crate::transcript::{TranscriptEditDeps, TranscriptEditService, TranscriptIndexer};

/// Leaves re-indexing to the test, which runs it when it wants to look.
struct ManualReindex;

impl TranscriptIndexer for ManualReindex {
    fn changed(&self, _meetings: &[MeetingId]) {}
}

/// A meeting indexed for both searches, its lines, and the edit service.
async fn indexed() -> (Harness, MeetingId, Vec<SegmentId>, TranscriptEditService) {
    let harness = Harness::new().await;
    let lines = [
        "the vault combination is zebra",
        "we ship the graph migration friday",
        "thanks everyone",
    ];
    let meeting = harness
        .meeting("meeting-sync", "Platform sync", &lines)
        .await;
    harness.indexer.index_meeting(&meeting).await.unwrap();
    let repo = Arc::new(SqliteSegmentRepo::new(harness.database.clone()));
    let ids = repo.list(&meeting, Page::ALL).await.unwrap();
    let ids = ids.into_iter().map(|segment| segment.id).collect();
    let service = TranscriptEditService::new(TranscriptEditDeps {
        editor: repo.clone(),
        writer: repo,
        indexer: Arc::new(ManualReindex),
        clock: Arc::new(FakeClock::new(at("2026-10-10T10:00:00Z"))),
    });
    (harness, meeting, ids, service)
}

/// Every text a search for `query` shows, keyword and semantic fused.
async fn found(harness: &Harness, query: &str) -> Vec<String> {
    let retriever = Retriever::open(
        harness.database.reader().clone(),
        Some(harness.embedder.clone()),
    );
    let hits = retriever
        .hybrid_search(query, &SearchFilters::default(), 20)
        .await
        .unwrap();
    hits.into_iter()
        .flat_map(|hit| [hit.snippet.text, hit.label.text])
        .collect()
}

fn mentions(texts: &[String], word: &str) -> bool {
    texts.iter().any(|text| text.contains(word))
}

#[tokio::test]
async fn a_deleted_line_is_never_found_again() {
    let (harness, meeting, ids, service) = indexed().await;
    assert!(mentions(&found(&harness, "zebra").await, "zebra"));

    service.delete(&ids[0]).await.unwrap();
    let at_once = found(&harness, "zebra combination").await;
    harness.indexer.index_meeting(&meeting).await.unwrap();
    let reindexed = found(&harness, "zebra combination").await;

    assert!(!mentions(&at_once, "zebra"), "{at_once:?}");
    assert!(!mentions(&reindexed, "zebra"), "{reindexed:?}");
    assert!(mentions(&found(&harness, "graph migration").await, "graph"));
}

#[tokio::test]
async fn replaced_words_leave_both_indexes_and_new_ones_arrive() {
    let (harness, meeting, ids, service) = indexed().await;

    service
        .edit(&ids[1], "we ship the graph migration monday")
        .await
        .unwrap();
    let at_once = found(&harness, "friday").await;
    harness.indexer.index_meeting(&meeting).await.unwrap();

    assert!(!mentions(&at_once, "friday"), "{at_once:?}");
    assert!(!mentions(&found(&harness, "friday").await, "friday"));
    assert!(mentions(&found(&harness, "monday").await, "monday"));
}
