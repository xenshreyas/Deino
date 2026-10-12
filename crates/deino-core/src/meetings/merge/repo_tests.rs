//! [`MeetingMergeService`] over the real schema: one transaction that moves
//! a meeting's transcript, speakers, people, tags, folders, action items
//! and notes into the earliest meeting, and keeps its recording reachable.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{FakeClock, MeetingId};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::test_seed as rows;
use crate::meetings::{MeetingError, RecordedWork, SqliteMeetingRepo};
use crate::notes::SqliteNotesRepo;

fn merger(database: &Database) -> MeetingMergeService {
    merger_with(database, Arc::new(RecordedWork::default()))
}

fn merger_with(database: &Database, work: Arc<RecordedWork>) -> MeetingMergeService {
    MeetingMergeService::new(MergeDeps {
        repo: Arc::new(SqliteMeetingRepo::new(database.clone())),
        notes: Arc::new(SqliteNotesRepo::new(database.clone())),
        clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
        work,
    })
}

/// Two finished meetings: `m0` (60 s, Maya invited) and a day later `m1`
/// (30 s, Maya organising, Lee speaking).
async fn two_meetings() -> (Database, String, String) {
    let db = test_db().await;
    seed::meeting(&db, "m0", "Kickoff", at("2026-10-01T09:00:00Z")).await;
    seed::meeting(&db, "m1", "Follow-up", at("2026-10-02T09:00:00Z")).await;
    rows::set(&db, "m0", ("duration_s", Some("60"))).await;
    rows::set(&db, "m1", ("duration_s", Some("30"))).await;
    let mic = Some("/data/recordings/m1/mic.m4a");
    rows::set(&db, "m1", ("audio_mic_path", mic)).await;
    let (maya, maya_0) = people_and_transcripts(&db).await;
    (db, maya, maya_0)
}

/// Maya in both meetings, Lee in `m1`, a line each; returns Maya and her
/// speaker in `m0`.
async fn people_and_transcripts(db: &Database) -> (String, String) {
    let maya = rows::person(db, 1, None, None).await;
    let lee = rows::person(db, 2, None, None).await;
    rows::attend(db, "m0", &maya, "invitee").await;
    rows::attend(db, "m1", &maya, "organizer").await;
    rows::attend(db, "m1", &lee, "speaker_only").await;
    let maya_0 = rows::speaker(db, 10, "m0", ("Speaker 1", Some(&maya))).await;
    let lee_1 = rows::speaker(db, 11, "m1", ("Speaker 1", Some(&lee))).await;
    let maya_1 = rows::speaker(db, 12, "m1", ("Speaker 2", Some(&maya))).await;
    rows::segment(db, 20, "m0", (0, 1_000, Some(&maya_0))).await;
    rows::segment(db, 21, "m1", (500, 1_500, Some(&lee_1))).await;
    rows::segment(db, 22, "m1", (2_000, 3_000, Some(&maya_1))).await;
    (maya, maya_0)
}

async fn scalar<T>(db: &Database, sql: &str) -> T
where
    T: Send + Unpin + for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
{
    sqlx::query_scalar(sql)
        .fetch_one(db.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn the_transcript_is_concatenated_with_offsets_onto_joined_speakers() {
    let (db, _, maya_0) = two_meetings().await;

    let outcome = merger(&db)
        .merge(&[MeetingId::new("m1"), MeetingId::new("m0")])
        .await
        .unwrap();

    assert_eq!(outcome.meeting_id, MeetingId::new("m0"));
    assert_eq!(outcome.absorbed, [MeetingId::new("m1")]);
    let lines: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT g.start_ms, s.label, g.words_json FROM transcript_segments AS g
         JOIN speakers AS s ON s.id = g.speaker_id WHERE g.meeting_id = 'm0' ORDER BY g.start_ms",
    )
    .fetch_all(db.reader())
    .await
    .unwrap();
    let starts: Vec<i64> = lines.iter().map(|line| line.0).collect();
    let labels: Vec<&str> = lines.iter().map(|line| line.1.as_str()).collect();
    assert_eq!(starts, [0, 60_500, 62_000]);
    assert_eq!(
        labels,
        ["Speaker 1", "Speaker 1 (2)", "Speaker 1"],
        "Maya's lines join her speaker"
    );
    assert!(
        lines[1].2.contains(r#""start":60500"#),
        "word timings shift too: {}",
        lines[1].2
    );
    let maya_lines: i64 = scalar(
        &db,
        &format!("SELECT COUNT(*) FROM transcript_segments WHERE speaker_id = '{maya_0}'"),
    )
    .await;
    assert_eq!(maya_lines, 2);
}

#[tokio::test]
async fn people_unite_with_the_stronger_role_and_links_move() {
    let (db, maya, _) = two_meetings().await;
    let tag = rows::tag(&db, 30, "Customer").await;
    rows::tag_meeting(&db, "m1", &tag).await;
    let folder = rows::folder(&db, 31, None).await;
    rows::file(&db, "m1", &folder).await;
    rows::action_item(&db, 32, "m1").await;

    merger(&db)
        .merge(&[MeetingId::new("m0"), MeetingId::new("m1")])
        .await
        .unwrap();

    let role: String = scalar(
        &db,
        &format!("SELECT role FROM meeting_attendees WHERE person_id = '{maya}'"),
    )
    .await;
    assert_eq!(role, "organizer");
    for (table, column) in [
        ("meeting_attendees", "meeting_id"),
        ("meeting_tags", "meeting_id"),
        ("meeting_folders", "meeting_id"),
        ("action_items", "meeting_id"),
    ] {
        let elsewhere: i64 = scalar(
            &db,
            &format!("SELECT COUNT(*) FROM {table} WHERE {column} <> 'm0'"),
        )
        .await;
        assert_eq!(elsewhere, 0, "{table} moved");
    }
    assert_eq!(scalar::<i64>(&db, "SELECT COUNT(*) FROM meetings").await, 1);
}

#[tokio::test]
async fn the_absorbed_recording_stays_reachable_and_the_length_covers_the_timeline() {
    let (db, ..) = two_meetings().await;
    seed::notes(&db, "m1", "Ship Friday").await;

    merger(&db)
        .merge(&[MeetingId::new("m0"), MeetingId::new("m1")])
        .await
        .unwrap();

    let part: (i64, String, i64, String) = sqlx::query_as(
        "SELECT position, source_meeting_id, offset_ms, audio_mic_path FROM meeting_merge_parts",
    )
    .fetch_one(db.reader())
    .await
    .unwrap();
    assert_eq!(
        part,
        (1, "m1".into(), 60_000, "/data/recordings/m1/mic.m4a".into())
    );
    assert_eq!(
        scalar::<i64>(&db, "SELECT duration_s FROM meetings WHERE id = 'm0'").await,
        90
    );
    let notes: String = scalar(
        &db,
        "SELECT notes_markdown FROM meeting_notes WHERE meeting_id = 'm0'",
    )
    .await;
    assert!(notes.starts_with("## Follow-up"), "{notes}");
}

#[tokio::test]
async fn unknown_or_unfinished_meetings_are_refused_and_nothing_changes() {
    let (db, ..) = two_meetings().await;
    rows::set(&db, "m1", ("status", Some("processing"))).await;
    let merger = merger(&db);

    let busy = merger
        .merge(&[MeetingId::new("m0"), MeetingId::new("m1")])
        .await
        .unwrap_err();
    let unknown = merger
        .merge(&[MeetingId::new("m0"), MeetingId::new("mx")])
        .await
        .unwrap_err();
    let single = merger
        .merge(&[MeetingId::new("m0"), MeetingId::new("m0")])
        .await
        .unwrap_err();

    assert!(matches!(busy, MeetingError::NotMergeable(id) if id.as_str() == "m1"));
    assert!(matches!(unknown, MeetingError::NotFound(_)));
    assert!(matches!(single, MeetingError::MergeNeedsTwo));
    assert_eq!(scalar::<i64>(&db, "SELECT COUNT(*) FROM meetings").await, 2);
}

#[tokio::test]
async fn the_merged_meeting_is_reindexed_and_a_refused_merge_reindexes_nothing() {
    let (db, ..) = two_meetings().await;
    let work = Arc::new(RecordedWork::default());
    let merger = merger_with(&db, work.clone());

    merger
        .merge(&[MeetingId::new("m0"), MeetingId::new("mx")])
        .await
        .unwrap_err();
    merger
        .merge(&[MeetingId::new("m1"), MeetingId::new("m0")])
        .await
        .unwrap();

    assert_eq!(work.reindexed(), [MeetingId::new("m0")]);
    assert!(work.cancelled().is_empty());
}
