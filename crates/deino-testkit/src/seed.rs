//! Inserts rows that tests need but no feature service can create yet.
//!
//! Each helper panics when its write fails, so a broken seed fails the
//! calling test at once. Whole fixture meetings are seeded by
//! [`FixtureSeeder`](crate::FixtureSeeder) instead.

#![allow(
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "test helpers: a broken seed must fail the calling test at once"
)]

use chrono::{DateTime, Utc};
use deino_db::Database;

/// Inserts a meeting as the legacy recording path saves it.
pub async fn meeting(database: &Database, id: &str, title: &str, created_at: DateTime<Utc>) {
    sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)")
        .bind(id)
        .bind(title)
        .bind(created_at)
        .execute(database.writer())
        .await
        .expect("the meeting row is valid");
}

/// Inserts the transcript chunk that summary generation keeps per meeting.
pub async fn transcript_chunk(database: &Database, meeting_id: &str, meeting_name: &str) {
    sqlx::query(
        "INSERT INTO transcript_chunks (meeting_id, meeting_name, transcript_text, model, model_name, created_at)
         VALUES (?1, ?2, '', 'builtin-ai', 'test', '2026-10-08T09:00:00Z')",
    )
    .bind(meeting_id)
    .bind(meeting_name)
    .execute(database.writer())
    .await
    .expect("the transcript chunk row is valid");
}
