//! The corrections port and its SQLite adapter: corrected segments and the
//! originals kept in `vocab_corrections`, so every correction can be undone.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{MeetingId, SegmentId};
use sqlx::SqliteConnection;
use sqlx::types::Json;

use super::{CorrectedSegment, Correction, UndoReport};
use crate::segments::WordTiming;

/// Which corrections to undo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoScope<'a> {
    /// Every correction of a meeting.
    Meeting(&'a MeetingId),
    /// One segment's correction.
    Segment(&'a SegmentId),
}

/// Where corrections and their originals are stored.
#[async_trait]
pub trait CorrectionRepo: Send + Sync {
    /// Corrects segments that still read `original_text` and keeps their
    /// originals, in one transaction; a segment whose correction was undone
    /// is left alone. Returns how many segments changed.
    async fn apply(&self, corrections: &[Correction], at: DateTime<Utc>) -> Result<u32, DbError>;

    /// Keeps the originals of live segments saved already corrected, and
    /// forgets those of `uncorrected` (a draft the vocabulary no longer
    /// changes), in one transaction.
    async fn record(
        &self,
        corrections: &[Correction],
        uncorrected: &[SegmentId],
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// The meeting's corrections that are not undone, in transcript order.
    async fn corrections(&self, meeting: &MeetingId) -> Result<Vec<CorrectedSegment>, DbError>;

    /// Restores the originals in `scope` the user has not edited since, and
    /// marks every correction in it undone, in one transaction.
    async fn undo(&self, scope: UndoScope<'_>, at: DateTime<Utc>) -> Result<UndoReport, DbError>;
}

/// [`CorrectionRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteCorrectionRepo {
    database: Database,
}

impl SqliteCorrectionRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// A kept original beside the segment's text now.
struct Kept {
    segment_id: SegmentId,
    original_text: String,
    original_words: Json<Vec<WordTiming>>,
    corrected_text: String,
    text: String,
}

#[async_trait]
impl CorrectionRepo for SqliteCorrectionRepo {
    async fn apply(&self, corrections: &[Correction], at: DateTime<Utc>) -> Result<u32, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let mut applied = 0;
        for correction in corrections {
            if correct_segment(&mut transaction, correction).await? {
                keep_first_original(&mut transaction, correction, at).await?;
                applied += 1;
            }
        }
        transaction.commit().await?;
        Ok(applied)
    }

    async fn record(
        &self,
        corrections: &[Correction],
        uncorrected: &[SegmentId],
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for correction in corrections {
            keep_latest_original(&mut transaction, correction, at).await?;
        }
        for id in uncorrected {
            sqlx::query!(
                "DELETE FROM vocab_corrections WHERE segment_id = ?1 AND undone_at IS NULL",
                id
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn corrections(&self, meeting: &MeetingId) -> Result<Vec<CorrectedSegment>, DbError> {
        let rows = sqlx::query_file_as!(
            Kept,
            "src/vocabulary/queries/kept.sql",
            meeting,
            None::<SegmentId>
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| CorrectedSegment {
                undoable: row.text == row.corrected_text,
                segment_id: row.segment_id,
                original_text: row.original_text,
                corrected_text: row.corrected_text,
            })
            .collect())
    }

    async fn undo(&self, scope: UndoScope<'_>, at: DateTime<Utc>) -> Result<UndoReport, DbError> {
        let (meeting, segment) = match scope {
            UndoScope::Meeting(meeting) => (Some(meeting), None),
            UndoScope::Segment(segment) => (None, Some(segment)),
        };
        let mut transaction = self.database.writer().begin().await?;
        let rows = sqlx::query_file_as!(Kept, "src/vocabulary/queries/kept.sql", meeting, segment)
            .fetch_all(&mut *transaction)
            .await?;
        let mut report = UndoReport::default();
        for row in rows {
            let restore = row.text == row.corrected_text;
            undo_one(&mut transaction, &row, restore, at).await?;
            *if restore {
                &mut report.restored
            } else {
                &mut report.kept
            } += 1;
        }
        transaction.commit().await?;
        Ok(report)
    }
}

/// Writes the corrected text and words over a segment that still reads the
/// original and whose correction was never undone; true when it did.
async fn correct_segment(
    connection: &mut SqliteConnection,
    correction: &Correction,
) -> Result<bool, DbError> {
    let words = Json(&correction.corrected_words);
    let result = sqlx::query!(
        "UPDATE transcript_segments SET text = ?1, words_json = ?2
         WHERE id = ?3 AND text = ?4
           AND NOT EXISTS (SELECT 1 FROM vocab_corrections
                           WHERE segment_id = ?3 AND undone_at IS NOT NULL)",
        correction.corrected_text,
        words,
        correction.segment_id,
        correction.original_text,
    )
    .execute(connection)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Keeps the original of a segment the stage corrected; a segment corrected
/// before keeps its first original (the engine's).
async fn keep_first_original(
    connection: &mut SqliteConnection,
    correction: &Correction,
    at: DateTime<Utc>,
) -> Result<(), DbError> {
    let words = Json(&correction.original_words);
    sqlx::query!(
        "INSERT INTO vocab_corrections (segment_id, original_text, original_words_json, corrected_text, corrected_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (segment_id) DO UPDATE SET
             corrected_text = excluded.corrected_text, corrected_at = excluded.corrected_at",
        correction.segment_id,
        correction.original_text,
        words,
        correction.corrected_text,
        at,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Keeps the original of a live segment; a draft that grew replaces it.
async fn keep_latest_original(
    connection: &mut SqliteConnection,
    correction: &Correction,
    at: DateTime<Utc>,
) -> Result<(), DbError> {
    let words = Json(&correction.original_words);
    sqlx::query!(
        "INSERT INTO vocab_corrections (segment_id, original_text, original_words_json, corrected_text, corrected_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (segment_id) DO UPDATE SET
             original_text = excluded.original_text, original_words_json = excluded.original_words_json,
             corrected_text = excluded.corrected_text, corrected_at = excluded.corrected_at",
        correction.segment_id,
        correction.original_text,
        words,
        correction.corrected_text,
        at,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Marks one correction undone, restoring the original when `restore`.
async fn undo_one(
    connection: &mut SqliteConnection,
    row: &Kept,
    restore: bool,
    at: DateTime<Utc>,
) -> Result<(), DbError> {
    if restore {
        sqlx::query!(
            "UPDATE transcript_segments SET text = ?1, words_json = ?2 WHERE id = ?3",
            row.original_text,
            row.original_words,
            row.segment_id,
        )
        .execute(&mut *connection)
        .await?;
    }
    sqlx::query!(
        "UPDATE vocab_corrections SET undone_at = ?1 WHERE segment_id = ?2",
        at,
        row.segment_id,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

#[cfg(test)]
#[path = "corrections_tests.rs"]
mod tests;
