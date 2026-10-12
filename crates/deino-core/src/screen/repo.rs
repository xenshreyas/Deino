//! [`ScreenRepo`] and its SQLite implementation over `snapshots` and
//! `screen_capture_choices`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{MeetingId, Millis};

use super::{NewSnapshot, Snapshot};

/// Screen capture's rows.
#[async_trait]
pub trait ScreenRepo: Send + Sync {
    /// Stores a kept snapshot. Storing the same file twice is a no-op.
    async fn add_snapshot(&self, snapshot: &NewSnapshot, at: DateTime<Utc>) -> Result<(), DbError>;

    /// Every snapshot of `meeting`, in time order.
    async fn snapshots(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError>;

    /// Records the user's choice for `meeting`: capture or not.
    async fn set_capture(
        &self,
        meeting: &MeetingId,
        capture: bool,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// The user's choice for `meeting`, if they made one.
    async fn capture_choice(&self, meeting: &MeetingId) -> Result<Option<bool>, DbError>;
}

/// [`ScreenRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteScreenRepo {
    database: Database,
}

impl SqliteScreenRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ScreenRepo for SqliteScreenRepo {
    async fn add_snapshot(&self, snapshot: &NewSnapshot, at: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO snapshots (meeting_id, at_ms, file_name, width, height, phash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (meeting_id, file_name) DO NOTHING",
            snapshot.meeting,
            snapshot.at,
            snapshot.file_name,
            snapshot.width,
            snapshot.height,
            snapshot.phash,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn snapshots(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError> {
        let rows = sqlx::query!(
            r#"SELECT id AS "id!: i64", at_ms AS "at: Millis", file_name,
                      width AS "width: u32", height AS "height: u32"
               FROM snapshots WHERE meeting_id = ?1 ORDER BY at_ms, id"#,
            meeting
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| Snapshot {
                id: row.id,
                at: row.at,
                file_name: row.file_name,
                width: row.width,
                height: row.height,
            })
            .collect())
    }

    async fn set_capture(
        &self,
        meeting: &MeetingId,
        capture: bool,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO screen_capture_choices (meeting_id, capture, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (meeting_id) DO UPDATE SET capture = excluded.capture, updated_at = excluded.updated_at",
            meeting,
            capture,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn capture_choice(&self, meeting: &MeetingId) -> Result<Option<bool>, DbError> {
        let row = sqlx::query_scalar!(
            r#"SELECT capture AS "capture: bool" FROM screen_capture_choices WHERE meeting_id = ?1"#,
            meeting
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(row)
    }
}

#[cfg(test)]
#[path = "repo_tests.rs"]
mod tests;
