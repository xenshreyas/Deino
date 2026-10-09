//! The meeting-digest port: what the auto-tagger reads about meetings.
//!
//! Defined by its one consumer, the auto-tagger, so it depends on a
//! meeting's title, summary and transcript opening and nothing else.

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, TagId};

use super::SqliteTagRepo;

/// A meeting as the auto-tagger sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingDigest {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// Its title.
    pub title: String,
    /// Its finished summary, in Markdown; `None` until one exists.
    pub summary: Option<String>,
}

/// Reads meeting digests for the auto-tagger.
#[async_trait]
pub trait MeetingDigestReader: Send + Sync {
    /// The meeting's title and summary; `None` when no meeting outside the
    /// trash has the ID.
    async fn digest(&self, meeting: &MeetingId) -> Result<Option<MeetingDigest>, DbError>;

    /// The opening of the meeting's transcript, at most `max_chars`
    /// characters of its first segments, for a meeting with no summary.
    async fn transcript_opening(
        &self,
        meeting: &MeetingId,
        max_chars: usize,
    ) -> Result<String, DbError>;

    /// Up to `limit` meetings outside the trash carrying `tag`, newest
    /// first, other than `except`.
    async fn examples(
        &self,
        tag: &TagId,
        except: &MeetingId,
        limit: u32,
    ) -> Result<Vec<MeetingDigest>, DbError>;
}

/// Segments read for a transcript opening; enough for a few thousand
/// characters without reading a whole meeting.
const OPENING_SEGMENTS: i64 = 200;

#[async_trait]
impl MeetingDigestReader for SqliteTagRepo {
    async fn digest(&self, meeting: &MeetingId) -> Result<Option<MeetingDigest>, DbError> {
        let digest = sqlx::query_file_as!(MeetingDigest, "src/tags/queries/digest.sql", meeting)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(digest)
    }

    async fn transcript_opening(
        &self,
        meeting: &MeetingId,
        max_chars: usize,
    ) -> Result<String, DbError> {
        let lines = sqlx::query_scalar!(
            "SELECT text FROM transcript_segments
             WHERE meeting_id = ?1 AND is_deleted = 0
             ORDER BY start_ms, id
             LIMIT ?2",
            meeting,
            OPENING_SEGMENTS,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(opening(&lines, max_chars))
    }

    async fn examples(
        &self,
        tag: &TagId,
        except: &MeetingId,
        limit: u32,
    ) -> Result<Vec<MeetingDigest>, DbError> {
        let examples = sqlx::query_file_as!(
            MeetingDigest,
            "src/tags/queries/examples.sql",
            tag,
            except,
            limit
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(examples)
    }
}

/// `lines` joined by spaces, cut to at most `max_chars` characters.
pub(super) fn opening(lines: &[String], max_chars: usize) -> String {
    let joined = lines.join(" ");
    joined.chars().take(max_chars).collect()
}
