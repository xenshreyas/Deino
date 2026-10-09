//! FND-07 migration round trip: a database in the pre-FND-07 format, seeded
//! the way the legacy code wrote it, migrates into the core schema without
//! losing a meeting or a segment, and the legacy SQL keeps working on top of
//! the `transcripts` compatibility view (ADR 0012).
//!
//! The technique is anarlog's `migration_steps_before` (MIT): apply only the
//! migrations older than FND-07, insert legacy rows, then run the full
//! [`MIGRATOR`].

use std::borrow::Cow;

use sqlx::migrate::Migrator;
use tokio::sync::broadcast::error::TryRecvError;

use super::*;

/// The version of `20261009060435_fnd07_core_schema.sql`.
const FND07_VERSION: i64 = 20_261_009_060_435;

/// Rows of the legacy `transcripts` shape: (id, meeting, text, start s, end s).
type LegacyRow = (
    &'static str,
    &'static str,
    &'static str,
    Option<f64>,
    Option<f64>,
);

const LEGACY_ROWS: [LegacyRow; 4] = [
    (
        "transcript-0b5fd3c2-1111-4222-8333-444455556666",
        "meeting-a",
        "second",
        Some(4.0),
        Some(61.2),
    ),
    (
        "transcript-1c6fd3c2-1111-4222-8333-444455556666",
        "meeting-a",
        "first",
        Some(1.25),
        Some(3.5),
    ),
    ("not-a-uuid", "meeting-b", "untimed", None, None),
    (
        "transcript-2d7fd3c2-1111-4222-8333-444455556666",
        "meeting-gone",
        "orphan",
        Some(0.0),
        Some(1.0),
    ),
];

/// `(id, status, source, duration_s, started_at, folder_path)` of a meeting.
type MeetingRow = (String, String, String, Option<i64>, Option<String>, String);

/// (id, transcript, timestamp, start s, end s, duration s) of the legacy view.
type LegacyTranscriptRow = (
    String,
    String,
    String,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

/// A migrator with only the migrations before `version`.
fn migrator_before(version: i64) -> Migrator {
    let migrations = MIGRATOR.iter().filter(|m| m.version < version).cloned();
    Migrator {
        migrations: Cow::Owned(migrations.collect()),
        ..Migrator::DEFAULT
    }
}

async fn execute(database: &Database, sql: &str) {
    sqlx::query(sql).execute(database.writer()).await.unwrap();
}

/// A pre-FND-07 database holding legacy meetings and transcripts, including
/// an orphan row left behind while foreign keys were off.
async fn legacy_database() -> Database {
    let database = Database::open(&DbLocation::Memory, &migrator_before(FND07_VERSION))
        .await
        .unwrap();
    for id in ["meeting-a", "meeting-b", "meeting-c"] {
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at, folder_path)
             VALUES (?1, 'Standup', '2026-10-01T09:00:00+00:00', '2026-10-01T09:30:00+00:00', '/tmp/a')",
        )
        .bind(id)
        .execute(database.writer())
        .await
        .unwrap();
    }
    execute(&database, "PRAGMA foreign_keys = OFF").await;
    for (id, meeting, text, start, end) in LEGACY_ROWS {
        sqlx::query(
            "INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration, speaker)
             VALUES (?1, ?2, ?3, '14:30:05', ?4, ?5, ?5 - ?4, 'mic')",
        )
        .bind(id)
        .bind(meeting)
        .bind(text)
        .bind(start)
        .bind(end)
        .execute(database.writer())
        .await
        .unwrap();
    }
    execute(&database, "PRAGMA foreign_keys = ON").await;
    database
}

async fn migrated_legacy_database() -> Database {
    let database = legacy_database().await;
    MIGRATOR.run(database.writer()).await.unwrap();
    database
}

/// (id, meeting, channel, start, end, text) of every segment, in table order.
async fn segments(database: &Database) -> Vec<(String, String, String, i64, i64, String)> {
    sqlx::query_as(
        "SELECT id, meeting_id, channel, start_ms, end_ms, text FROM transcript_segments ORDER BY rowid",
    )
    .fetch_all(database.reader())
    .await
    .unwrap()
}

#[tokio::test]
async fn legacy_transcripts_become_mixed_segments_with_their_uuids_and_timing() {
    let database = migrated_legacy_database().await;

    let rows = segments(&database).await;

    let summary: Vec<(&str, &str, i64, i64, &str)> = rows
        .iter()
        .map(|(_, meeting, channel, start, end, text)| {
            (
                meeting.as_str(),
                channel.as_str(),
                *start,
                *end,
                text.as_str(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("meeting-a", "mixed", 4_000, 61_200, "second"),
            ("meeting-a", "mixed", 1_250, 3_500, "first"),
            ("meeting-b", "mixed", 0, 0, "untimed"),
        ],
        "the orphan of a deleted meeting is dropped"
    );
    assert_eq!(rows[0].0, "0b5fd3c2-1111-4222-8333-444455556666");
    let fresh = &rows[2].0;
    assert_eq!(fresh.len(), 36, "a non-UUID legacy ID gets a fresh UUID");
    assert_eq!(fresh.chars().nth(14), Some('4'), "the fresh UUID is a v4");
}

#[tokio::test]
async fn existing_meetings_are_ready_live_and_keep_every_legacy_column() {
    let database = migrated_legacy_database().await;

    let meetings: Vec<MeetingRow> = sqlx::query_as(
        "SELECT id, status, source, duration_s, started_at, folder_path FROM meetings ORDER BY id",
    )
    .fetch_all(database.reader())
    .await
    .unwrap();

    let ready = |id: &str, duration: Option<i64>| {
        (
            id.into(),
            "ready".into(),
            "live".into(),
            duration,
            None,
            "/tmp/a".into(),
        )
    };
    assert_eq!(
        meetings,
        [
            ready("meeting-a", Some(61)),
            ready("meeting-b", Some(0)),
            ready("meeting-c", None),
        ]
    );
}

#[tokio::test]
async fn the_compatibility_view_serves_the_legacy_transcript_page_query() {
    let database = migrated_legacy_database().await;

    // The legacy meeting page's query (`get_meeting_transcripts_paginated`).
    let page: Vec<LegacyTranscriptRow> =
        sqlx::query_as(
            "SELECT id, transcript, timestamp, audio_start_time, audio_end_time, duration
             FROM transcripts WHERE meeting_id = ?1 ORDER BY audio_start_time ASC LIMIT ?2 OFFSET ?3",
        )
        .bind("meeting-a")
        .bind(10)
        .bind(0)
        .fetch_all(database.reader())
        .await
        .unwrap();

    let texts: Vec<(&str, &str, Option<f64>)> = page
        .iter()
        .map(|(_, text, timestamp, start, ..)| (text.as_str(), timestamp.as_str(), *start))
        .collect();
    assert_eq!(
        texts,
        [
            ("first", "00:00:01", Some(1.25)),
            ("second", "00:00:04", Some(4.0))
        ]
    );
    assert_eq!(page[1].5, Some(57.2));
}

#[tokio::test]
async fn legacy_writes_through_the_view_land_in_segments_and_signal_them() {
    let database = migrated_legacy_database().await;
    let mut changes = database.changes().subscribe();

    // The exact statements of the legacy import and re-transcription paths.
    let mut transaction = database.writer().begin().await.unwrap();
    sqlx::query("DELETE FROM transcripts WHERE meeting_id = ?")
        .bind("meeting-a")
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("transcript-3e8fd3c2-1111-4222-8333-444455556666")
    .bind("meeting-a")
    .bind("redone")
    .bind("2026-10-09T06:00:00+00:00")
    .bind(2.0)
    .bind(2.5)
    .bind(0.5)
    .execute(&mut *transaction)
    .await
    .unwrap();
    transaction.commit().await.unwrap();

    let rows = segments(&database).await;
    let meeting_a: Vec<_> = rows.iter().filter(|row| row.1 == "meeting-a").collect();
    assert_eq!(meeting_a.len(), 1);
    assert_eq!(meeting_a[0].0, "3e8fd3c2-1111-4222-8333-444455556666");
    assert_eq!((meeting_a[0].3, meeting_a[0].4), (2_000, 2_500));
    let signal = changes.try_recv().unwrap();
    assert_eq!(signal.tables, [Table::TranscriptSegments]);
    assert_eq!(changes.try_recv().unwrap_err(), TryRecvError::Empty);
}

#[tokio::test]
async fn deleting_a_meeting_cascades_to_its_segments() {
    let database = migrated_legacy_database().await;

    execute(&database, "DELETE FROM meetings WHERE id = 'meeting-a'").await;

    let remaining: Vec<String> = segments(&database)
        .await
        .into_iter()
        .map(|row| row.1)
        .collect();
    assert_eq!(remaining, ["meeting-b"]);
}
