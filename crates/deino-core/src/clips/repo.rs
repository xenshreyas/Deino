//! [`ClipRepo`]: the `clips` table. Reads join `meetings` for the title and
//! never return a trashed meeting's clips.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{ClipId, MeetingId, Millis};

use super::{Clip, ClipMedia, ClipRename, ClipRow};

/// Where clips are stored.
#[async_trait]
pub trait ClipRepo: Send + Sync {
    /// One clip, unless it or its meeting is gone or trashed.
    async fn find(&self, id: &ClipId) -> Result<Option<Clip>, DbError>;

    /// A meeting's clips in time order.
    async fn list_for_meeting(&self, meeting: &MeetingId) -> Result<Vec<Clip>, DbError>;

    /// Every clip of every meeting not in the trash, newest first.
    async fn list_all(&self) -> Result<Vec<Clip>, DbError>;

    /// Every path a clip row names, for unique file names.
    async fn paths_like(&self, folder_prefix: &str) -> Result<Vec<String>, DbError>;

    /// Saves a new clip.
    async fn insert(&self, row: &ClipRow) -> Result<(), DbError>;

    /// Renames a clip and points it at its new file; `false` if it is gone.
    async fn rename(&self, id: &ClipId, change: &ClipRename) -> Result<bool, DbError>;

    /// Deletes a clip's row; `false` if it was already gone.
    async fn delete(&self, id: &ClipId) -> Result<bool, DbError>;
}

/// [`ClipRepo`] over SQLite.
#[derive(Clone, Debug)]
pub struct SqliteClipRepo {
    database: Database,
}

impl SqliteClipRepo {
    /// The repository over the app's database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ClipRepo for SqliteClipRepo {
    async fn find(&self, id: &ClipId) -> Result<Option<Clip>, DbError> {
        let clip = sqlx::query_as!(
            Clip,
            r#"SELECT c.id AS "id: ClipId", c.meeting_id AS "meeting_id: MeetingId",
                m.title AS meeting_title, c.name, c.start_ms AS "start: Millis",
                c.end_ms AS "end: Millis", c.media AS "media: ClipMedia", c.path,
                c.created_at AS "created_at: DateTime<Utc>"
            FROM clips c JOIN meetings m ON m.id = c.meeting_id
            WHERE c.id = ?1 AND m.deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(clip)
    }

    async fn list_for_meeting(&self, meeting: &MeetingId) -> Result<Vec<Clip>, DbError> {
        let clips = sqlx::query_as!(
            Clip,
            r#"SELECT c.id AS "id: ClipId", c.meeting_id AS "meeting_id: MeetingId",
                m.title AS meeting_title, c.name, c.start_ms AS "start: Millis",
                c.end_ms AS "end: Millis", c.media AS "media: ClipMedia", c.path,
                c.created_at AS "created_at: DateTime<Utc>"
            FROM clips c JOIN meetings m ON m.id = c.meeting_id
            WHERE c.meeting_id = ?1 AND m.deleted_at IS NULL
            ORDER BY c.start_ms, c.id"#,
            meeting,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(clips)
    }

    async fn list_all(&self) -> Result<Vec<Clip>, DbError> {
        let clips = sqlx::query_as!(
            Clip,
            r#"SELECT c.id AS "id: ClipId", c.meeting_id AS "meeting_id: MeetingId",
                m.title AS meeting_title, c.name, c.start_ms AS "start: Millis",
                c.end_ms AS "end: Millis", c.media AS "media: ClipMedia", c.path,
                c.created_at AS "created_at: DateTime<Utc>"
            FROM clips c JOIN meetings m ON m.id = c.meeting_id
            WHERE m.deleted_at IS NULL
            ORDER BY c.created_at DESC, c.id DESC"#,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(clips)
    }

    async fn paths_like(&self, folder_prefix: &str) -> Result<Vec<String>, DbError> {
        let pattern = format!("{}%", escape_like(folder_prefix));
        let paths = sqlx::query_scalar!(
            r"SELECT path FROM clips WHERE path LIKE ?1 ESCAPE '\'",
            pattern,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(paths)
    }

    async fn insert(&self, row: &ClipRow) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO clips (id, meeting_id, name, start_ms, end_ms, media, path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            row.id,
            row.meeting_id,
            row.name,
            row.start,
            row.end,
            row.media,
            row.path,
            row.at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn rename(&self, id: &ClipId, change: &ClipRename) -> Result<bool, DbError> {
        let done = sqlx::query!(
            "UPDATE clips SET name = ?2, path = ?3, updated_at = ?4 WHERE id = ?1",
            id,
            change.name,
            change.path,
            change.at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn delete(&self, id: &ClipId) -> Result<bool, DbError> {
        let done = sqlx::query!("DELETE FROM clips WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(done.rows_affected() > 0)
    }
}

/// `text` with LIKE's wildcards and the escape character escaped.
fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
