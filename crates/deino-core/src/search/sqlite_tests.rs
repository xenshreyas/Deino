//! The keyword index against a migrated database: the triggers keep it in
//! step with every source, trashed meetings never match, and the queries
//! use the full-text index.

use deino_db::Database;
use deino_domain::{MeetingId, Millis};
use deino_testkit::{at, seed, test_db};

use super::*;
use crate::search::{FilterChip, Highlight, fts, query};
use crate::testing::query_plan;

const SEGMENT_A: &str = "0192f3a0-0000-7000-8000-00000000000a";
const SEGMENT_B: &str = "0192f3a0-0000-7000-8000-00000000000b";

async fn execute(database: &Database, sql: &str) {
    sqlx::query(sql).execute(database.writer()).await.unwrap();
}

/// Two meetings: a roadmap review with a transcript, and a standup.
async fn seeded() -> Database {
    let database = test_db().await;
    seed::meeting(
        &database,
        "meeting-a",
        "Q4 roadmap review",
        at("2026-10-08T09:00:00Z"),
    )
    .await;
    seed::meeting(
        &database,
        "meeting-b",
        "Daily standup",
        at("2026-10-09T09:00:00Z"),
    )
    .await;
    execute(
        &database,
        &format!(
            "INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text) VALUES
             ('{SEGMENT_A}', 'meeting-a', 'mixed', 61000, 64000, 'We should ship the calendar sync beta in November'),
             ('{SEGMENT_B}', 'meeting-b', 'mixed', 1000, 3000, 'Blocked on the jobs endpoint')"
        ),
    )
    .await;
    database
}

fn index(database: &Database) -> SqliteSearchIndex {
    SqliteSearchIndex::new(database)
}

async fn search(database: &Database, text: &str) -> Vec<Hit> {
    search_in(database, text, None).await
}

async fn search_in(database: &Database, text: &str, scope: Option<&[MeetingId]>) -> Vec<Hit> {
    let parsed = query::parse(text);
    let expression = fts::match_expression(&parsed.terms, parsed.open_ended).unwrap();
    let query = KeywordQuery {
        expression: &expression,
        scope,
        limit: 20,
    };
    index(database).keyword(query).await.unwrap()
}

fn refs(hits: &[Hit]) -> Vec<(HitKind, &str)> {
    hits.iter()
        .map(|hit| (hit.kind, hit.ref_id.as_str()))
        .collect()
}

#[tokio::test]
async fn existing_rows_are_indexed_and_a_moment_carries_its_time_and_highlights() {
    let database = seeded().await;

    let hits = search(&database, "calendar beta ").await;

    assert_eq!(refs(&hits), [(HitKind::Segment, SEGMENT_A)]);
    let moment = &hits[0];
    assert_eq!(moment.meeting_id, Some(MeetingId::new("meeting-a")));
    assert_eq!(moment.ts_ms, Some(Millis(61_000)));
    assert_eq!(moment.label.text, "Q4 roadmap review");
    assert!(moment.label.highlights.is_empty());
    assert_eq!(
        moment.snippet.text,
        "We should ship the calendar sync beta in November"
    );
    assert_eq!(
        moment.snippet.highlights,
        [
            Highlight { start: 19, end: 27 },
            Highlight { start: 33, end: 37 }
        ]
    );
    assert!(moment.score > 0.0);
}

#[tokio::test]
async fn stemming_and_the_open_last_word_match_like_the_user_means() {
    let database = seeded().await;

    assert_eq!(
        refs(&search(&database, "shipping").await),
        [(HitKind::Segment, SEGMENT_A)]
    );
    assert_eq!(
        refs(&search(&database, "roadm").await),
        [(HitKind::Meeting, "meeting-a")]
    );
    assert!(
        search(&database, "roadm ").await.is_empty(),
        "a finished word is not a prefix"
    );
    let title = &search(&database, "roadmap").await[0].label;
    assert_eq!(title.highlights, [Highlight { start: 3, end: 10 }]);
}

#[tokio::test]
async fn edits_renames_and_deletes_reach_the_index() {
    let database = seeded().await;

    execute(
        &database,
        &format!(
            "UPDATE transcript_segments SET text = 'Moved to December' WHERE id = '{SEGMENT_A}'"
        ),
    )
    .await;
    execute(
        &database,
        "UPDATE meetings SET title = 'Planning offsite' WHERE id = 'meeting-b'",
    )
    .await;
    assert!(search(&database, "November").await.is_empty());
    assert_eq!(
        refs(&search(&database, "December").await),
        [(HitKind::Segment, SEGMENT_A)]
    );
    assert_eq!(
        refs(&search(&database, "offsite").await),
        [(HitKind::Meeting, "meeting-b")]
    );
    assert!(search(&database, "standup").await.is_empty());

    execute(
        &database,
        &format!("DELETE FROM transcript_segments WHERE id = '{SEGMENT_A}'"),
    )
    .await;
    assert!(search(&database, "December").await.is_empty());
    execute(&database, "DELETE FROM meetings WHERE id = 'meeting-b'").await;
    assert!(
        search(&database, "jobs").await.is_empty(),
        "a deleted meeting's moments go"
    );
}

#[tokio::test]
async fn soft_deleted_segments_and_trashed_meetings_never_match() {
    let database = seeded().await;

    execute(
        &database,
        &format!("UPDATE transcript_segments SET is_deleted = 1 WHERE id = '{SEGMENT_B}'"),
    )
    .await;
    assert!(search(&database, "jobs").await.is_empty());
    execute(
        &database,
        &format!("UPDATE transcript_segments SET is_deleted = 0 WHERE id = '{SEGMENT_B}'"),
    )
    .await;
    assert_eq!(search(&database, "jobs").await.len(), 1);

    execute(
        &database,
        "UPDATE meetings SET deleted_at = '2026-10-10T00:00:00Z' WHERE id = 'meeting-a'",
    )
    .await;
    assert!(search(&database, "calendar").await.is_empty());
    assert!(search(&database, "roadmap").await.is_empty());
    execute(
        &database,
        "UPDATE meetings SET deleted_at = NULL WHERE id = 'meeting-a'",
    )
    .await;
    assert_eq!(
        search(&database, "calendar").await.len(),
        1,
        "restoring needs no reindex"
    );
}

/// Writes version `version` of a meeting's enhanced notes (NOT-02).
async fn enhanced_notes(database: &Database, meeting: &str, version: i64, markdown: &str) {
    sqlx::query(
        r#"INSERT INTO enhanced_notes
            (meeting_id, version, doc_json, markdown, model, created_at, updated_at)
        VALUES (?1, ?2, '{"type":"doc","content":[]}', ?3, 'test:model', '', '')"#,
    )
    .bind(meeting)
    .bind(version)
    .bind(markdown)
    .execute(database.writer())
    .await
    .unwrap();
}

/// Notes and two versions of enhanced notes for meeting-a, an action item
/// and a person.
async fn with_entities(database: &Database) {
    execute(
        database,
        "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at)
        VALUES ('meeting-a', '# Risks\n- Graph renewals', '', '')",
    )
    .await;
    enhanced_notes(database, "meeting-a", 1, "Decided to postpone").await;
    enhanced_notes(database, "meeting-a", 2, "Decided to hire").await;
    execute(database, "INSERT INTO action_items (id, meeting_id, title, created_by, created_at, updated_at)
        VALUES ('0192f3a0-0000-7000-8000-0000000000c1', 'meeting-a', 'Send the deck', 'user', '', '')").await;
    execute(
        database,
        "INSERT INTO people (id, name, email, created_at, updated_at)
        VALUES ('0192f3a0-0000-7000-8000-0000000000d1', 'Maya Chen', 'maya@deino.com', '', '')",
    )
    .await;
}

#[tokio::test]
async fn notes_summaries_action_items_and_people_are_indexed() {
    let database = seeded().await;
    with_entities(&database).await;

    assert_eq!(
        refs(&search(&database, "renewals").await),
        [(HitKind::Note, "meeting-a")]
    );
    assert_eq!(
        refs(&search(&database, "hire").await),
        [(HitKind::Summary, "meeting-a")]
    );
    assert_eq!(
        refs(&search(&database, "deck").await)[0].0,
        HitKind::ActionItem
    );
    assert_eq!(refs(&search(&database, "maya").await)[0].0, HitKind::Person);
}

#[tokio::test]
async fn emptied_notes_leave_the_index_and_renamed_people_follow() {
    let database = seeded().await;
    with_entities(&database).await;

    execute(
        &database,
        "UPDATE meeting_notes SET notes_markdown = '' WHERE meeting_id = 'meeting-a'",
    )
    .await;
    execute(
        &database,
        "UPDATE people SET name = 'Maya Lee' WHERE email = 'maya@deino.com'",
    )
    .await;

    assert!(search(&database, "renewals").await.is_empty());
    assert_eq!(search(&database, "lee").await[0].label.text, "Maya Lee");
    execute(
        &database,
        "INSERT INTO people (id, name, created_at, updated_at)
        VALUES ('0192f3a0-0000-7000-8000-0000000000d2', 'Maya C.', '', '')",
    )
    .await;
    execute(
        &database,
        "UPDATE people SET merged_into = '0192f3a0-0000-7000-8000-0000000000d2'
        WHERE email = 'maya@deino.com'",
    )
    .await;
    assert!(
        search(&database, "lee").await.is_empty(),
        "merged people leave the index"
    );
}

#[tokio::test]
async fn a_scope_limits_hits_to_its_meetings_and_drops_entities() {
    let database = seeded().await;
    execute(
        &database,
        "INSERT INTO people (id, name, created_at, updated_at)
                        VALUES ('0192f3a0-0000-7000-8000-0000000000d2', 'Blocked Person', '', '')",
    )
    .await;

    let scope = [MeetingId::new("meeting-a")];
    assert!(
        search_in(&database, "blocked", Some(&scope))
            .await
            .is_empty()
    );
    assert_eq!(search(&database, "blocked").await.len(), 2);
}

#[tokio::test]
async fn refresh_and_reindex_rebuild_the_same_documents() {
    let database = seeded().await;
    execute(
        &database,
        "DELETE FROM search_documents WHERE meeting_id = 'meeting-a'",
    )
    .await;
    assert!(search(&database, "calendar").await.is_empty());

    index(&database)
        .refresh_meeting(&MeetingId::new("meeting-a"))
        .await
        .unwrap();
    assert_eq!(search(&database, "calendar").await.len(), 1);

    let report = index(&database).reindex_all().await.unwrap();
    assert_eq!(report.documents, 4, "two titles and two segments");
    assert_eq!(search(&database, "calendar").await.len(), 1);
}

const ANA: &str = "0192f3a0-0000-7000-8000-0000000000e1";
const BO: &str = "0192f3a0-0000-7000-8000-0000000000e2";

/// Ana (eu.acme.com) attended meeting-a; Bo (other.org) both meetings;
/// meeting-a is tagged and filed in a subfolder of "Customers".
async fn with_attendees(database: &Database) {
    execute(
        database,
        &format!(
            "INSERT INTO people (id, name, email, created_at, updated_at) VALUES
        ('{ANA}', 'Ana', 'ana@eu.acme.com', '', ''), ('{BO}', 'Bo', 'bo@other.org', '', '')"
        ),
    )
    .await;
    execute(database, &format!("INSERT INTO meeting_attendees (meeting_id, person_id, role) VALUES
        ('meeting-a', '{ANA}', 'invitee'), ('meeting-a', '{BO}', 'invitee'), ('meeting-b', '{BO}', 'invitee')")).await;
    execute(
        database,
        "INSERT INTO tags (id, name, color, created_at, updated_at)
        VALUES ('0192f3a0-0000-7000-8000-0000000000b1', 'Renewal', 1, '', '')",
    )
    .await;
    execute(
        database,
        "INSERT INTO meeting_tags (meeting_id, tag_id, applied_by, applied_at)
        VALUES ('meeting-a', '0192f3a0-0000-7000-8000-0000000000b1', 'user', '')",
    )
    .await;
    execute(database, "INSERT INTO folders (id, parent_id, name, created_at, updated_at) VALUES
        ('0192f3a0-0000-7000-8000-0000000000f1', NULL, 'Customers', '', ''),
        ('0192f3a0-0000-7000-8000-0000000000f2', '0192f3a0-0000-7000-8000-0000000000f1', 'Acme', '', '')").await;
    execute(
        database,
        "INSERT INTO meeting_folders (meeting_id, folder_id, added_at)
        VALUES ('meeting-a', '0192f3a0-0000-7000-8000-0000000000f2', '')",
    )
    .await;
}

fn chips(chips: &[(FilterKind, &str)]) -> SearchFilters {
    let chips = chips
        .iter()
        .map(|(kind, id)| FilterChip {
            kind: *kind,
            id: (*id).to_owned(),
            label: String::new(),
        })
        .collect();
    SearchFilters {
        chips,
        domains: Vec::new(),
    }
}

async fn scope_ids(database: &Database, filters: SearchFilters) -> Vec<String> {
    let meetings = index(database).scope(&filters).await.unwrap();
    meetings
        .into_iter()
        .map(|meeting| meeting.id.as_str().to_owned())
        .collect()
}

#[tokio::test]
async fn the_scope_requires_every_person_and_lists_newest_first() {
    let database = seeded().await;
    with_attendees(&database).await;

    let bo = chips(&[(FilterKind::Person, BO)]);
    let both = chips(&[(FilterKind::Person, BO), (FilterKind::Person, ANA)]);

    assert_eq!(scope_ids(&database, bo).await, ["meeting-b", "meeting-a"]);
    assert_eq!(scope_ids(&database, both).await, ["meeting-a"]);
}

#[tokio::test]
async fn domains_match_attendee_emails_and_subdomains_only() {
    let database = seeded().await;
    with_attendees(&database).await;
    let domain = |domain: &str| SearchFilters {
        chips: Vec::new(),
        domains: vec![domain.to_owned()],
    };

    assert_eq!(
        scope_ids(&database, domain("acme.com")).await,
        ["meeting-a"]
    );
    assert!(scope_ids(&database, domain("cme.com")).await.is_empty());
}

#[tokio::test]
async fn tags_folders_with_their_subfolders_and_meetings_narrow_the_scope() {
    let database = seeded().await;
    with_attendees(&database).await;

    let tag = chips(&[(FilterKind::Tag, "0192f3a0-0000-7000-8000-0000000000b1")]);
    let parent = chips(&[(FilterKind::Folder, "0192f3a0-0000-7000-8000-0000000000f1")]);
    let meeting = chips(&[(FilterKind::Meeting, "meeting-b")]);

    assert_eq!(scope_ids(&database, tag).await, ["meeting-a"]);
    assert_eq!(scope_ids(&database, parent).await, ["meeting-a"]);
    assert_eq!(scope_ids(&database, meeting).await, ["meeting-b"]);
}

#[tokio::test]
async fn keyword_search_is_served_by_the_full_text_index() {
    let database = seeded().await;

    let plan = query_plan(&database, include_str!("queries/keyword.sql")).await;

    assert!(
        plan.iter().any(|step| step.contains("VIRTUAL TABLE INDEX")),
        "{plan:?}"
    );
    assert!(
        plan.iter()
            .any(|step| step.contains("SEARCH d USING INTEGER PRIMARY KEY")),
        "{plan:?}"
    );
}

#[tokio::test]
async fn trigger_lookups_use_the_kind_and_ref_index() {
    let database = seeded().await;

    let plan = query_plan(
        &database,
        "DELETE FROM search_documents WHERE kind = ?1 AND ref_id = ?2",
    )
    .await;

    assert!(
        plan.iter()
            .any(|step| step.contains("sqlite_autoindex_search_documents_1")),
        "{plan:?}"
    );
}

#[tokio::test]
async fn only_the_newest_enhanced_notes_version_is_searchable() {
    let database = seeded().await;
    with_entities(&database).await;

    assert!(search(&database, "postpone").await.is_empty());
    execute(
        &database,
        "UPDATE enhanced_notes SET markdown = 'Decided to wait' WHERE meeting_id = 'meeting-a' AND version = 2",
    )
    .await;
    assert!(search(&database, "hire").await.is_empty());
    assert_eq!(
        refs(&search(&database, "wait").await),
        [(HitKind::Summary, "meeting-a")]
    );
    execute(
        &database,
        "DELETE FROM enhanced_notes WHERE meeting_id = 'meeting-a' AND version = 2",
    )
    .await;
    assert_eq!(
        refs(&search(&database, "postpone").await),
        [(HitKind::Summary, "meeting-a")],
        "deleting the newest version falls back to the one before"
    );
}
