//! [`SegmentEditor`]'s contract against the SQLite repository and the fake,
//! and what only the schema does: the trash, meeting order, and the TRN-02
//! triggers (stale embedding windows, the legacy view's delete).

use deino_db::{Database, test_db};
use deino_domain::{Channel, FakeClock, MeetingId, Millis, SegmentId};
use deino_testkit::{at, seed};

use super::*;

const STANDUP: &str = "meeting-standup";
const PLANNING: &str = "meeting-planning";

fn segment(meeting: &str, start: u32, text: &str) -> NewSegment {
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    NewSegment {
        id: SegmentId::generate(&clock),
        meeting_id: MeetingId::new(meeting),
        channel: Channel::System,
        speaker_id: None,
        start: Millis(start),
        end: Millis(start + 900),
        text: text.into(),
        words: Vec::new(),
    }
}

async fn database() -> Database {
    let database = test_db().await;
    seed::meeting(&database, PLANNING, "Planning", at("2026-10-07T09:00:00Z")).await;
    seed::meeting(&database, STANDUP, "Standup", at("2026-10-08T09:00:00Z")).await;
    database
}

/// One segment reads back whatever its state; batched edits keep the first original.
async fn edits_keep_the_engines_text(repo: &(impl SegmentEditor + SegmentWriter)) {
    let line = segment(STANDUP, 0, "We ship Monday.");
    repo.append(std::slice::from_ref(&line)).await.unwrap();
    let edit = |text: &str| TextEdit {
        id: line.id,
        text: text.into(),
    };

    repo.edit_many(&[edit("We ship Tuesday.")], at("2026-10-09T10:00:00Z"))
        .await
        .unwrap();
    repo.edit_many(&[edit("We ship Friday.")], at("2026-10-09T11:00:00Z"))
        .await
        .unwrap();
    repo.set_deleted(&line.id, true).await.unwrap();

    let stored = repo.segment(&line.id).await.unwrap().unwrap();
    assert_eq!(stored.text, "We ship Friday.");
    assert_eq!(stored.original_text.as_deref(), Some("We ship Monday."));
    assert_eq!(stored.edited_at, Some(at("2026-10-09T11:00:00Z")));
    assert!(stored.is_deleted);
}

/// Matching finds live lines case-insensitively, literally, in scope.
async fn matching_finds_live_lines_in_scope(repo: &(impl SegmentEditor + SegmentWriter)) {
    let deleted = segment(STANDUP, 2_000, "DINO deleted");
    repo.append(&[
        segment(STANDUP, 0, "Dino at 50% off"),
        segment(STANDUP, 1_000, "nothing"),
        deleted.clone(),
        segment(PLANNING, 0, "dino planning"),
    ])
    .await
    .unwrap();
    repo.set_deleted(&deleted.id, true).await.unwrap();

    let standup = MeetingId::new(STANDUP);
    let mine = repo.matching(Some(&standup), "dino").await.unwrap();
    let everywhere = repo.matching(None, "DINO").await.unwrap();
    let literal = repo.matching(None, "50%").await.unwrap();
    let wildcard = repo.matching(None, "_").await.unwrap();

    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].text, "Dino at 50% off");
    assert_eq!(everywhere.len(), 2);
    assert_eq!(literal.len(), 1);
    assert!(wildcard.is_empty());
}

#[tokio::test]
async fn sqlite_edits_keep_the_engines_text() {
    edits_keep_the_engines_text(&SqliteSegmentRepo::new(database().await)).await;
}

#[tokio::test]
async fn fake_edits_keep_the_engines_text() {
    edits_keep_the_engines_text(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn sqlite_matching_finds_live_lines_in_scope() {
    matching_finds_live_lines_in_scope(&SqliteSegmentRepo::new(database().await)).await;
}

#[tokio::test]
async fn fake_matching_finds_live_lines_in_scope() {
    matching_finds_live_lines_in_scope(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn matching_skips_the_trash_and_lists_the_newest_meeting_first_with_titles() {
    let database = database().await;
    seed::meeting(
        &database,
        "meeting-trashed",
        "Old",
        at("2026-10-09T09:00:00Z"),
    )
    .await;
    sqlx::query(
        "UPDATE meetings SET deleted_at = '2026-10-10T00:00:00Z' WHERE id = 'meeting-trashed'",
    )
    .execute(database.writer())
    .await
    .unwrap();
    let repo = SqliteSegmentRepo::new(database);
    repo.append(&[
        segment(PLANNING, 0, "roadmap"),
        segment(STANDUP, 0, "roadmap"),
        segment("meeting-trashed", 0, "roadmap"),
    ])
    .await
    .unwrap();

    let found = repo.matching(None, "roadmap").await.unwrap();

    let titles: Vec<&str> = found
        .iter()
        .map(|line| line.meeting_title.as_str())
        .collect();
    assert_eq!(titles, ["Standup", "Planning"]);
}

/// Inserts one transcript window of `meeting` over `[start, end]`.
async fn window(database: &Database, chunk: i64, (start, end): (i64, i64)) {
    sqlx::query(
        "INSERT INTO embeddings (meeting_id, ref_kind, ref_id, chunk_ix, ts_ms, end_ms, text, model, vector, created_at)
         VALUES (?1, 'transcript', 'segment', ?2, ?3, ?4, 'window text', 'model', x'00000000', '2026-10-08T09:00:00Z')",
    )
    .bind(STANDUP)
    .bind(chunk)
    .bind(start)
    .bind(end)
    .execute(database.writer())
    .await
    .unwrap();
}

async fn windows(database: &Database) -> Vec<i64> {
    sqlx::query_scalar("SELECT chunk_ix FROM embeddings ORDER BY chunk_ix")
        .fetch_all(database.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn editing_or_deleting_a_line_drops_the_embedding_windows_over_it() {
    let database = database().await;
    let repo = SqliteSegmentRepo::new(database.clone());
    let (edited, deleted) = (segment(STANDUP, 0, "a"), segment(STANDUP, 10_000, "b"));
    repo.append(&[edited.clone(), deleted.clone()])
        .await
        .unwrap();
    for (chunk, span) in [(0, (0, 5_000)), (1, (9_000, 12_000)), (2, (20_000, 30_000))] {
        window(&database, chunk, span).await;
    }

    repo.edit_text(&edited.id, "A", at("2026-10-09T10:00:00Z"))
        .await
        .unwrap();
    repo.set_deleted(&deleted.id, true).await.unwrap();

    assert_eq!(windows(&database).await, [2]);
}

#[tokio::test]
async fn diarization_rewriting_text_without_an_edit_keeps_the_windows() {
    let database = database().await;
    let repo = SqliteSegmentRepo::new(database.clone());
    let line = segment(STANDUP, 0, "a b");
    repo.append(std::slice::from_ref(&line)).await.unwrap();
    window(&database, 0, (0, 5_000)).await;

    sqlx::query("UPDATE transcript_segments SET text = 'a' WHERE id = ?1")
        .bind(line.id)
        .execute(database.writer())
        .await
        .unwrap();

    assert_eq!(windows(&database).await, [0]);
}

#[tokio::test]
async fn deleting_through_the_legacy_view_also_removes_soft_deleted_lines() {
    let database = database().await;
    let repo = SqliteSegmentRepo::new(database.clone());
    let (kept, hidden) = (segment(STANDUP, 0, "a"), segment(STANDUP, 1_000, "b"));
    repo.append(&[kept, hidden.clone(), segment(PLANNING, 0, "c")])
        .await
        .unwrap();
    repo.set_deleted(&hidden.id, true).await.unwrap();

    sqlx::query("DELETE FROM transcripts WHERE meeting_id = ?1")
        .bind(STANDUP)
        .execute(database.writer())
        .await
        .unwrap();

    let left: Vec<String> = sqlx::query_scalar("SELECT meeting_id FROM transcript_segments")
        .fetch_all(database.reader())
        .await
        .unwrap();
    assert_eq!(left, [PLANNING]);
}
