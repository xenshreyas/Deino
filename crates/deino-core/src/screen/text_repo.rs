//! The snapshots' text over SQLite: [`SnapshotTextRepo`] (written by the
//! `screen_context` stage) and [`ScreenTexts`] (read by NOT-02's material),
//! both on [`SqliteScreenRepo`].

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, Millis};

use super::{ScreenText, Snapshot, SqliteScreenRepo};

/// The snapshots' text, written by the `screen_context` stage.
#[async_trait]
pub trait SnapshotTextRepo: Send + Sync {
    /// The snapshots of `meeting` not read yet, in time order.
    async fn unread(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError>;

    /// The snapshots of `meeting` read but not described, in time order.
    async fn undescribed(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError>;

    /// Stores what was read in snapshot `id` (`""`: no text).
    async fn set_text(&self, id: i64, text: &str) -> Result<(), DbError>;

    /// Stores snapshot `id`'s description.
    async fn set_description(&self, id: i64, description: &str) -> Result<(), DbError>;
}

/// What the notes read of the shared screen (NOT-02's material).
#[async_trait]
pub trait ScreenTexts: Send + Sync {
    /// The read snapshots of `meeting` with text or a description, in
    /// time order.
    async fn screen_texts(&self, meeting: &MeetingId) -> Result<Vec<ScreenText>, DbError>;
}

/// No screen text: tests, and sources without a screen.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoScreenTexts;

#[async_trait]
impl ScreenTexts for NoScreenTexts {
    async fn screen_texts(&self, _meeting: &MeetingId) -> Result<Vec<ScreenText>, DbError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl SnapshotTextRepo for SqliteScreenRepo {
    async fn unread(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError> {
        let rows = sqlx::query_as!(
            Snapshot,
            r#"SELECT id AS "id!: i64", at_ms AS "at: Millis", file_name,
                      width AS "width: u32", height AS "height: u32", ocr_text AS text
               FROM snapshots WHERE meeting_id = ?1 AND ocr_text IS NULL
               ORDER BY at_ms, id"#,
            meeting
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows)
    }

    async fn undescribed(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError> {
        let rows = sqlx::query_as!(
            Snapshot,
            r#"SELECT id AS "id!: i64", at_ms AS "at: Millis", file_name,
                      width AS "width: u32", height AS "height: u32", ocr_text AS text
               FROM snapshots
               WHERE meeting_id = ?1 AND ocr_text IS NOT NULL AND description IS NULL
               ORDER BY at_ms, id"#,
            meeting
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows)
    }

    async fn set_text(&self, id: i64, text: &str) -> Result<(), DbError> {
        sqlx::query!("UPDATE snapshots SET ocr_text = ?2 WHERE id = ?1", id, text)
            .execute(self.database.writer())
            .await?;
        Ok(())
    }

    async fn set_description(&self, id: i64, description: &str) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE snapshots SET description = ?2 WHERE id = ?1",
            id,
            description
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }
}

#[async_trait]
impl ScreenTexts for SqliteScreenRepo {
    async fn screen_texts(&self, meeting: &MeetingId) -> Result<Vec<ScreenText>, DbError> {
        let rows = sqlx::query!(
            r#"SELECT at_ms AS "at: Millis", ocr_text AS "text!: String", description
               FROM snapshots
               WHERE meeting_id = ?1 AND ocr_text IS NOT NULL
                 AND (trim(ocr_text) <> '' OR description IS NOT NULL)
               ORDER BY at_ms, id"#,
            meeting
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ScreenText {
                at: row.at,
                text: row.text,
                description: row.description,
            })
            .collect())
    }
}
