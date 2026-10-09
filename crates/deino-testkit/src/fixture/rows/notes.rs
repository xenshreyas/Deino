//! A fixture's `meeting_notes` row: the notes the user typed (`userNotes`).

use async_trait::async_trait;
use deino_db::DbError;
use deino_doc::{Flavor, from_markdown, to_markdown};
use sqlx::SqliteConnection;

use super::super::model::FixtureMeeting;
use super::super::seeding::SeedTarget;

/// The user's notes as the notepad stores them: the fixture's Markdown
/// parsed into a TipTap document (`notes_json`) and written back as Readable
/// Markdown (`notes_markdown`), saved when recording stopped. A fixture
/// without notes has no row, and re-seeding restores typed-over notes.
pub struct NoteRows;

#[async_trait]
impl SeedTarget for NoteRows {
    async fn seed(
        &self,
        connection: &mut SqliteConnection,
        meeting: &FixtureMeeting,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "DELETE FROM meeting_notes WHERE meeting_id = ?1",
            meeting.id
        )
        .execute(&mut *connection)
        .await?;
        let Some(markdown) = &meeting.user_notes else {
            return Ok(());
        };
        let doc = from_markdown(markdown).map_err(decode_error)?;
        let json = serde_json::to_string(&doc).map_err(decode_error)?;
        let readable = to_markdown(&doc, Flavor::Readable);
        let saved_at = meeting.ended_at();
        sqlx::query!(
            "INSERT INTO meeting_notes (meeting_id, notes_markdown, notes_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            meeting.id,
            readable,
            json,
            saved_at,
        )
        .execute(&mut *connection)
        .await?;
        Ok(())
    }
}

/// Fixture notes that do not convert fail the seed like a bad row would.
fn decode_error(error: impl std::error::Error + Send + Sync + 'static) -> DbError {
    DbError::Query(sqlx::Error::Decode(Box::new(error)))
}
