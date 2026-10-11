//! The service over the real keyword index and a scripted semantic search.

use std::sync::Arc;

use deino_db::Database;
use deino_domain::{MeetingId, Millis};
use deino_testkit::{at, seed, test_db};

use super::*;
use crate::search::{FakeSemanticSearch, SqliteSearchIndex};

const MAYA: &str = "0192f3a0-0000-7000-8000-0000000000f1";

async fn execute(database: &Database, sql: &str) {
    sqlx::query(sql).execute(database.writer()).await.unwrap();
}

/// A roadmap meeting Maya (acme.com) attended, and a standup she did not.
async fn seeded() -> Database {
    let database = test_db().await;
    seed::meeting(
        &database,
        "meeting-a",
        "Roadmap review",
        at("2026-10-08T09:00:00Z"),
    )
    .await;
    seed::meeting(
        &database,
        "meeting-b",
        "Standup roadmap",
        at("2026-10-09T09:00:00Z"),
    )
    .await;
    execute(
        &database,
        &format!(
            "INSERT INTO people (id, name, email, created_at, updated_at)
        VALUES ('{MAYA}', 'Maya Chen', 'maya@acme.com', '', '')"
        ),
    )
    .await;
    execute(
        &database,
        &format!(
            "INSERT INTO meeting_attendees (meeting_id, person_id, role)
        VALUES ('meeting-a', '{MAYA}', 'invitee')"
        ),
    )
    .await;
    execute(&database, "INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text) VALUES
        ('0192f3a0-0000-7000-8000-0000000000a1', 'meeting-a', 'mixed', 4000, 5000, 'the roadmap slipped')").await;
    database
}

fn service(database: &Database, semantic: Arc<FakeSemanticSearch>) -> SearchService {
    let index = Arc::new(SqliteSearchIndex::new(database));
    SearchService::new(index.clone(), index, semantic)
}

fn request(text: &str) -> SearchRequest {
    SearchRequest {
        text: text.to_owned(),
        filters: SearchFilters::default(),
        limit: 10,
    }
}

fn meeting_refs(results: &SearchResults) -> Vec<&str> {
    results
        .meetings
        .iter()
        .map(|hit| hit.meeting_id.as_ref().unwrap().as_str())
        .collect()
}

#[tokio::test]
async fn a_mention_that_names_someone_becomes_a_filter_chip() {
    let database = seeded().await;
    let search = service(&database, Arc::default());

    let results = search.search(&request("roadmap @maya")).await.unwrap();

    assert_eq!(meeting_refs(&results), ["meeting-a"]);
    assert_eq!(results.filters.chips[0].label, "Maya Chen");
    assert_eq!(results.filters.chips[0].kind, FilterKind::Person);
    assert!(
        results.people.is_empty(),
        "entities are not meeting content"
    );
    assert!(!results.semantic);
}

#[tokio::test]
async fn unknown_mentions_and_domains_are_searched_as_words() {
    let database = seeded().await;
    let search = service(&database, Arc::default());

    let results = search
        .search(&request("@roadmap unknown.org"))
        .await
        .unwrap();

    assert!(results.filters.is_empty());
    assert!(results.meetings.is_empty(), "both words must match");
    let roadmap = search.search(&request("@roadmap")).await.unwrap();
    assert_eq!(meeting_refs(&roadmap).len(), 2);
}

#[tokio::test]
async fn a_known_domain_alone_lists_its_meetings_newest_first() {
    let database = seeded().await;
    let search = service(&database, Arc::default());

    let results = search.search(&request("acme.com")).await.unwrap();

    assert_eq!(results.filters.domains, ["acme.com"]);
    assert_eq!(meeting_refs(&results), ["meeting-a"]);
    assert_eq!(results.meetings[0].kind, HitKind::Meeting);
}

#[tokio::test]
async fn semantic_hits_are_fused_in_and_searched_within_the_filters() {
    let database = seeded().await;
    let window = Hit {
        kind: HitKind::Segment,
        ref_id: "w".into(),
        meeting_id: Some(MeetingId::new("meeting-a")),
        ts_ms: Some(Millis(0)),
        end_ms: Some(Millis(9_000)),
        label: Snippet::plain("Roadmap review"),
        snippet: Snippet::plain("window"),
        date: None,
        score: 0.9,
    };
    let semantic = Arc::new(FakeSemanticSearch::answering(vec![window]));
    let search = service(&database, semantic.clone());

    let results = search.search(&request("slipped @maya")).await.unwrap();

    assert!(results.semantic);
    assert_eq!(
        results.moments.len(),
        1,
        "the keyword moment fused with its window"
    );
    assert_eq!(results.moments[0].snippet.text, "the roadmap slipped");
    assert_eq!(semantic.scopes(), [Some(vec![MeetingId::new("meeting-a")])]);
}

#[tokio::test]
async fn hybrid_search_returns_the_k_best_fused_hits() {
    let database = seeded().await;
    let search = service(&database, Arc::default());

    let hits = search
        .search_hybrid("roadmap", &SearchFilters::default(), 2)
        .await
        .unwrap();

    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|hit| hit.kind.is_meeting_content()));
}

#[tokio::test]
async fn filter_suggestions_match_name_prefixes_of_any_word() {
    let database = seeded().await;
    let search = service(&database, Arc::default());

    let chips = search.suggest_filters("che", None).await.unwrap();
    let meetings = search
        .suggest_filters("road", Some(FilterKind::Meeting))
        .await
        .unwrap();

    assert_eq!(chips.len(), 1);
    assert_eq!(chips[0].id, MAYA);
    let labels: Vec<&str> = meetings.iter().map(|chip| chip.label.as_str()).collect();
    assert_eq!(
        labels,
        ["Roadmap review", "Standup roadmap"],
        "the first or a later word"
    );
}
