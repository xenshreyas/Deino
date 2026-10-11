//! The transcript segment ports and their SQLite implementation.
//!
//! Reading and writing are separate ports ([`SegmentReader`],
//! [`SegmentWriter`]) so the transcript view and the transcription worker each
//! depend only on what they use. Writes of many segments run in one
//! transaction on the writer: the live recorder (FND-05b) appends each batch
//! of finalized segments with one commit, so a crash loses at most the batch
//! in flight and the UI receives one `DbChanged` per batch, not per row.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{Channel, MeetingId, Millis, SegmentId, SpeakerId};
use sqlx::SqliteConnection;
use sqlx::types::Json;

use super::{NewSegment, Page, Reattribution, Segment, WordTiming, voices};

/// Reads a meeting's transcript.
#[async_trait]
pub trait SegmentReader: Send + Sync {
    /// The meeting's segments that are not deleted, in time order (ties by
    /// ID), windowed by `page`.
    async fn list(&self, meeting: &MeetingId, page: Page) -> Result<Vec<Segment>, DbError>;

    /// How many segments [`SegmentReader::list`] would return without a page.
    async fn count(&self, meeting: &MeetingId) -> Result<u32, DbError>;
}

/// Writes a meeting's transcript.
#[async_trait]
pub trait SegmentWriter: Send + Sync {
    /// Stores `segments` in one transaction: all of them or none.
    async fn append(&self, segments: &[NewSegment]) -> Result<(), DbError>;

    /// Replaces every segment of `meeting` with `segments` in one transaction
    /// (re-transcription).
    async fn replace(&self, meeting: &MeetingId, segments: &[NewSegment]) -> Result<(), DbError>;

    /// Sets the segment's text after a user edit, stamping `edited_at`.
    async fn edit_text(
        &self,
        id: &SegmentId,
        text: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Hides the segment from the transcript, or brings it back.
    async fn set_deleted(&self, id: &SegmentId, deleted: bool) -> Result<bool, DbError>;

    /// Attributes the segment to a speaker of the same meeting, or to nobody.
    /// A speaker of another meeting is rejected by the schema.
    async fn set_speaker(
        &self,
        id: &SegmentId,
        speaker: Option<&SpeakerId>,
    ) -> Result<bool, DbError>;

    /// Re-attributes segments to speakers in one transaction (CAP-03): each
    /// segment becomes its head piece and gains its tail pieces as new
    /// segments on its meeting and channel. `edited_at` and `is_deleted`
    /// are left alone. A change to a missing segment is a no-op.
    async fn reattribute(&self, changes: &[Reattribution]) -> Result<(), DbError>;

    /// Saves the live transcript's changes in one transaction (FND-05b):
    /// inserts each of `segments`, or, when its ID is already stored (an
    /// open segment saved as a draft), replaces its timing, text and words;
    /// then deletes `removed` (drafts discarded as echo).
    async fn save_live(
        &self,
        segments: &[NewSegment],
        removed: &[SegmentId],
    ) -> Result<(), DbError>;
}

/// [`SegmentReader`], [`SegmentWriter`] and [`SpeakerRepo`](super::SpeakerRepo)
/// over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteSegmentRepo {
    pub(super) database: Database,
}

impl SqliteSegmentRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl SegmentReader for SqliteSegmentRepo {
    async fn list(&self, meeting: &MeetingId, page: Page) -> Result<Vec<Segment>, DbError> {
        let rows = sqlx::query_file_as!(
            SegmentRow,
            "src/segments/queries/list.sql",
            meeting,
            page.limit,
            page.offset,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(Segment::from).collect())
    }

    async fn count(&self, meeting: &MeetingId) -> Result<u32, DbError> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!: u32" FROM transcript_segments
            WHERE meeting_id = ?1 AND is_deleted = 0"#,
            meeting,
        )
        .fetch_one(self.database.reader())
        .await?;
        Ok(count)
    }
}

#[async_trait]
impl SegmentWriter for SqliteSegmentRepo {
    async fn append(&self, segments: &[NewSegment]) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for segment in segments {
            insert(&mut transaction, segment).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn replace(&self, meeting: &MeetingId, segments: &[NewSegment]) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        sqlx::query!(
            "DELETE FROM transcript_segments WHERE meeting_id = ?1",
            meeting
        )
        .execute(&mut *transaction)
        .await?;
        for segment in segments {
            insert(&mut transaction, segment).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn edit_text(
        &self,
        id: &SegmentId,
        text: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE transcript_segments SET text = ?1, edited_at = ?2 WHERE id = ?3",
            text,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_deleted(&self, id: &SegmentId, deleted: bool) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE transcript_segments SET is_deleted = ?1 WHERE id = ?2",
            deleted,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_speaker(
        &self,
        id: &SegmentId,
        speaker: Option<&SpeakerId>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE transcript_segments SET speaker_id = ?1 WHERE id = ?2",
            speaker,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn reattribute(&self, changes: &[Reattribution]) -> Result<(), DbError> {
        voices::reattribute(&self.database, changes).await
    }

    async fn save_live(
        &self,
        segments: &[NewSegment],
        removed: &[SegmentId],
    ) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for segment in segments {
            upsert(&mut transaction, segment).await?;
        }
        for id in removed {
            sqlx::query!("DELETE FROM transcript_segments WHERE id = ?1", id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}

/// Inserts one segment, or updates the live fields of the stored draft with
/// its ID, inside the caller's transaction.
async fn upsert(connection: &mut SqliteConnection, segment: &NewSegment) -> Result<(), DbError> {
    let words = Json(&segment.words);
    sqlx::query!(
        "INSERT INTO transcript_segments (id, meeting_id, channel, speaker_id, start_ms, end_ms, text, words_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (id) DO UPDATE SET
             start_ms = excluded.start_ms, end_ms = excluded.end_ms,
             text = excluded.text, words_json = excluded.words_json",
        segment.id,
        segment.meeting_id,
        segment.channel,
        segment.speaker_id,
        segment.start,
        segment.end,
        segment.text,
        words,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Inserts one segment inside the caller's transaction. The statement is
/// prepared once per connection and reused for every row of a batch.
async fn insert(connection: &mut SqliteConnection, segment: &NewSegment) -> Result<(), DbError> {
    let words = Json(&segment.words);
    sqlx::query!(
        "INSERT INTO transcript_segments (id, meeting_id, channel, speaker_id, start_ms, end_ms, text, words_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        segment.id,
        segment.meeting_id,
        segment.channel,
        segment.speaker_id,
        segment.start,
        segment.end,
        segment.text,
        words,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// One `transcript_segments` row as the list query reads it.
struct SegmentRow {
    id: SegmentId,
    meeting_id: MeetingId,
    channel: Channel,
    speaker_id: Option<SpeakerId>,
    start: Millis,
    end: Millis,
    text: String,
    words: Json<Vec<WordTiming>>,
    is_deleted: bool,
    edited_at: Option<DateTime<Utc>>,
}

impl From<SegmentRow> for Segment {
    fn from(row: SegmentRow) -> Self {
        Self {
            id: row.id,
            meeting_id: row.meeting_id,
            channel: row.channel,
            speaker_id: row.speaker_id,
            start: row.start,
            end: row.end,
            text: row.text,
            words: row.words.0,
            is_deleted: row.is_deleted,
            edited_at: row.edited_at,
        }
    }
}
