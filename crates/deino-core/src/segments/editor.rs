//! [`SegmentEditor`]: what the transcript edits (TRN-02) need beyond
//! [`SegmentWriter`](super::SegmentWriter): one segment by ID whatever its
//! state, many text edits in one transaction (find and replace), and the
//! segments a find and replace may change.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{Channel, MeetingId, Millis, SegmentId, SpeakerId};
use sqlx::types::Json;

use super::repo::SegmentRow;
use super::{Segment, SqliteSegmentRepo, WordTiming};

/// The escape character of the LIKE patterns [`like_pattern`] builds.
const LIKE_ESCAPE: char = '\\';

/// A new text for one segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// The segment.
    pub id: SegmentId,
    /// Its new text.
    pub text: String,
}

/// A segment a find and replace may change, with its meeting's title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentText {
    /// The segment.
    pub id: SegmentId,
    /// Its meeting.
    pub meeting_id: MeetingId,
    /// The meeting's title, for the preview.
    pub meeting_title: String,
    /// The segment's text now.
    pub text: String,
}

/// Reads and edits segments for the transcript edits.
#[async_trait]
pub trait SegmentEditor: Send + Sync {
    /// The segment with `id`, deleted or not; `None` when there is none.
    async fn segment(&self, id: &SegmentId) -> Result<Option<Segment>, DbError>;

    /// Applies every edit in one transaction, stamping `edited_at` and
    /// keeping each segment's first `original_text`. Edits of missing
    /// segments change nothing.
    async fn edit_many(&self, edits: &[TextEdit], at: DateTime<Utc>) -> Result<(), DbError>;

    /// Segments outside the trash and not deleted, of `meeting` or of every
    /// meeting, whose text may contain `needle`: every segment that contains
    /// it, compared case-insensitively for ASCII letters, newest meeting
    /// first, then in spoken order. Callers narrow a case-sensitive search.
    async fn matching(
        &self,
        meeting: Option<&MeetingId>,
        needle: &str,
    ) -> Result<Vec<SegmentText>, DbError>;
}

#[async_trait]
impl SegmentEditor for SqliteSegmentRepo {
    async fn segment(&self, id: &SegmentId) -> Result<Option<Segment>, DbError> {
        let row = sqlx::query_file_as!(SegmentRow, "src/segments/queries/one.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(row.map(Segment::from))
    }

    async fn edit_many(&self, edits: &[TextEdit], at: DateTime<Utc>) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for edit in edits {
            sqlx::query!(
                "UPDATE transcript_segments
                 SET original_text = coalesce(original_text, text), text = ?1, edited_at = ?2
                 WHERE id = ?3",
                edit.text,
                at,
                edit.id,
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn matching(
        &self,
        meeting: Option<&MeetingId>,
        needle: &str,
    ) -> Result<Vec<SegmentText>, DbError> {
        let pattern = like_pattern(needle);
        let rows = sqlx::query_file!("src/segments/queries/matching.sql", meeting, pattern)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| SegmentText {
                id: row.id,
                meeting_id: row.meeting_id,
                meeting_title: row.meeting_title,
                text: row.text,
            })
            .collect())
    }
}

/// `%needle%` for `LIKE … ESCAPE '\'`, with `%`, `_` and `\` taken literally.
pub(super) fn like_pattern(needle: &str) -> String {
    let mut pattern = String::with_capacity(needle.len() + 2);
    pattern.push('%');
    for character in needle.chars() {
        if matches!(character, '%' | '_' | LIKE_ESCAPE) {
            pattern.push(LIKE_ESCAPE);
        }
        pattern.push(character);
    }
    pattern.push('%');
    pattern
}
