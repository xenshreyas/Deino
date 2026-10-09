//! The notes port and its SQLite adapter over `meeting_notes`.
//!
//! `notes_json` holds the TipTap document (an opaque payload, never queried
//! by field, §3A) and `notes_markdown` the Markdown derived from it. The
//! adapter parses the JSON into [`Doc`] at this edge, so the rest of the
//! crate only sees typed documents.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_doc::Doc;
use deino_domain::{MeetingId, MeetingStatus};
use sqlx::SqliteConnection;

use super::StoredNotes;

/// Where meeting notes are stored.
#[async_trait]
pub trait NotesRepo: Send + Sync {
    /// The meeting's notes, if any were saved.
    async fn get(&self, meeting: &MeetingId) -> Result<Option<StoredNotes>, DbError>;

    /// Creates or replaces the meeting's notes, keeping the first `created_at`.
    async fn save(&self, notes: &StoredNotes) -> Result<(), DbError>;

    /// Saves `notes` and deletes the scheduled meeting `draft` (with its own
    /// notes) in one transaction. Returns `false`, changing nothing, when
    /// `draft` is missing or no longer scheduled.
    async fn transfer(&self, notes: &StoredNotes, draft: &MeetingId) -> Result<bool, DbError>;
}

/// [`NotesRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteNotesRepo {
    pub(super) database: Database,
}

impl SqliteNotesRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl NotesRepo for SqliteNotesRepo {
    async fn get(&self, meeting: &MeetingId) -> Result<Option<StoredNotes>, DbError> {
        let row = sqlx::query_file_as!(NotesRow, "src/notes/queries/get.sql", meeting)
            .fetch_optional(self.database.reader())
            .await?;
        row.map(StoredNotes::try_from).transpose()
    }

    async fn save(&self, notes: &StoredNotes) -> Result<(), DbError> {
        let mut connection = self.database.writer().acquire().await?;
        upsert(&mut connection, notes).await
    }

    async fn transfer(&self, notes: &StoredNotes, draft: &MeetingId) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let scheduled = MeetingStatus::Scheduled;
        let deleted = sqlx::query!(
            "DELETE FROM meetings WHERE id = ?1 AND status = ?2",
            draft,
            scheduled,
        )
        .execute(&mut *transaction)
        .await?;
        if deleted.rows_affected() == 0 {
            return Ok(false);
        }
        upsert(&mut transaction, notes).await?;
        transaction.commit().await?;
        Ok(true)
    }
}

/// The one write: insert, or replace everything but `created_at`.
async fn upsert(connection: &mut SqliteConnection, notes: &StoredNotes) -> Result<(), DbError> {
    let json = serde_json::to_string(&notes.doc).map_err(decode_error)?;
    sqlx::query!(
        "INSERT INTO meeting_notes (meeting_id, notes_markdown, notes_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (meeting_id) DO UPDATE SET
             notes_markdown = excluded.notes_markdown,
             notes_json = excluded.notes_json,
             updated_at = excluded.updated_at",
        notes.meeting_id,
        notes.markdown,
        json,
        notes.created_at,
        notes.updated_at,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// One `meeting_notes` row as stored.
struct NotesRow {
    meeting_id: MeetingId,
    notes_json: Option<String>,
    notes_markdown: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<NotesRow> for StoredNotes {
    type Error = DbError;

    /// Parses the stored document. A row from before the notepad, with only
    /// Markdown, is read through [`deino_doc::from_markdown`].
    fn try_from(row: NotesRow) -> Result<Self, DbError> {
        let markdown = row.notes_markdown.unwrap_or_default();
        let doc = match row.notes_json {
            Some(json) => serde_json::from_str::<Doc>(&json).map_err(decode_error)?,
            None => deino_doc::from_markdown(&markdown).map_err(decode_error)?,
        };
        Ok(Self {
            meeting_id: row.meeting_id,
            doc,
            markdown,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// A stored document that does not parse is a decode failure of the row.
fn decode_error(error: impl std::error::Error + Send + Sync + 'static) -> DbError {
    DbError::Query(sqlx::Error::Decode(Box::new(error)))
}
