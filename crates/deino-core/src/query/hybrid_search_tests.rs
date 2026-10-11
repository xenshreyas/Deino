//! `HybridSearch` over the sample: the app's index, from another process.

use deino_domain::{FixedZone, MeetingId};
use deino_testkit::test_db;

use super::*;
use crate::query::seed_sample_library;
use crate::query::testing::STANDUP;
use crate::search::FakeEmbedder;

async fn search_over(embedder: Option<Arc<dyn Embedder>>) -> HybridSearch {
    let database = test_db().await;
    let zone = Arc::new(FixedZone::hours(0).unwrap());
    seed_sample_library(&database, zone.as_ref()).await.unwrap();
    HybridSearch::new(database.reader().clone(), embedder, zone)
}

fn query(text: &str, meeting: Option<&str>) -> SearchQuery {
    SearchQuery {
        text: text.into(),
        meeting: meeting.map(MeetingId::new),
        limit: 0,
    }
}

#[tokio::test]
async fn a_blank_query_finds_nothing() {
    let search = search_over(None).await;

    let outcome = search.search(&query("   ", None)).await.unwrap();

    assert!(outcome.hits.is_empty());
}

#[tokio::test]
async fn transcript_hits_are_moments_with_links_inside_the_meeting() {
    let search = search_over(None).await;

    let outcome = search
        .search(&query("release cut", Some(STANDUP)))
        .await
        .unwrap();

    let moments: Vec<&SearchHit> = outcome
        .hits
        .iter()
        .filter(|hit| hit.kind == HitKind::Transcript)
        .collect();
    assert!(!moments.is_empty());
    assert!(
        outcome
            .hits
            .iter()
            .all(|hit| hit.meeting_id.as_ref().map(MeetingId::as_str) == Some(STANDUP))
    );
    let at = moments[0].at_ms.unwrap();
    assert_eq!(
        moments[0].link.as_deref(),
        Some(format!("deino://meeting/{STANDUP}?t={at}").as_str())
    );
}

#[tokio::test]
async fn without_an_embedder_the_search_is_keyword_only() {
    let search = search_over(None).await;

    let outcome = search.search(&query("postmortem", None)).await.unwrap();

    assert_eq!(outcome.mode, SearchMode::Keyword);
    assert!(
        outcome
            .hits
            .iter()
            .any(|hit| hit.kind == HitKind::ActionItem)
    );
}

#[tokio::test]
async fn an_embedder_that_answers_makes_it_hybrid_and_one_that_cannot_does_not() {
    let embedder = Arc::new(FakeEmbedder::default());
    let search = search_over(Some(embedder.clone())).await;

    let hybrid = search
        .search(&query("on-call rotation", None))
        .await
        .unwrap();
    embedder.set_ready(false);
    let keyword = search
        .search(&query("on-call rotation", None))
        .await
        .unwrap();

    assert_eq!(hybrid.mode, SearchMode::Hybrid);
    assert_eq!(keyword.mode, SearchMode::Keyword);
}
