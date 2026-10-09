//! The meeting trash port and its SQLite implementation.
//!
//! Trashing sets `deleted_at`, which hides the meeting from
//! [`MeetingRepo::list`](super::MeetingRepo::list) but keeps every row that
//! belongs to it; deleting removes the meeting for good, and foreign keys
//! cascade to its transcript, speakers, attendees, folders and tags.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::MeetingId;

use super::{Meeting, SqliteMeetingRepo};

/// Moves meetings into and out of the trash, and deletes them.
#[async_trait]
pub trait MeetingTrashRepo: Send + Sync {
    /// Moves the meeting to the trash at `at`. Returns `false` when the
    /// meeting is missing or already in the trash.
    async fn trash(&self, id: &MeetingId, at: DateTime<Utc>) -> Result<bool, DbError>;

    /// Takes the meeting out of the trash. Returns `false` when the meeting
    /// is missing or not in the trash.
    async fn restore(&self, id: &MeetingId, at: DateTime<Utc>) -> Result<bool, DbError>;

    /// Meetings in the trash, most recently trashed first.
    async fn list_trashed(&self) -> Result<Vec<Meeting>, DbError>;

    /// Deletes the meeting and everything that belongs to it, for good.
    /// Returns `false` when no meeting has `id`.
    async fn delete(&self, id: &MeetingId) -> Result<bool, DbError>;
}

#[async_trait]
impl MeetingTrashRepo for SqliteMeetingRepo {
    async fn trash(&self, id: &MeetingId, at: DateTime<Utc>) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE meetings SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn restore(&self, id: &MeetingId, at: DateTime<Utc>) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE meetings SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NOT NULL",
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_trashed(&self) -> Result<Vec<Meeting>, DbError> {
        let meetings = sqlx::query_file_as!(Meeting, "src/meetings/queries/list_trashed.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(meetings)
    }

    async fn delete(&self, id: &MeetingId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM meetings WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
