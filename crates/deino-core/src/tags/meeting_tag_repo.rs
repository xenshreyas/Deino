//! The meeting-tag port: which tags are on which meetings.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{MeetingId, PaletteColor, TagId, TagSource};

use super::{AppliedTag, MeetingTagLink, SqliteTagRepo, Tag, TagApplication};
use crate::meetings::Meeting;

/// Puts tags on meetings and takes them off.
#[async_trait]
pub trait MeetingTagRepo: Send + Sync {
    /// Puts the tag on the meeting and returns true. Applying a tag the
    /// meeting already has changes nothing and returns false, so the first
    /// applier is kept (a user's manual tag stays manual when the auto-tagger
    /// agrees). Fails if the meeting or the tag does not exist.
    async fn apply_tag(&self, application: &TagApplication) -> Result<bool, DbError>;

    /// Takes the tag off the meeting.
    async fn remove_tag(&self, meeting: &MeetingId, tag: &TagId) -> Result<bool, DbError>;

    /// The tags on the meeting, by name, with who applied them.
    async fn tags_of(&self, meeting: &MeetingId) -> Result<Vec<AppliedTag>, DbError>;

    /// The meetings outside the trash carrying the tag, newest first.
    async fn meetings_tagged(&self, tag: &TagId) -> Result<Vec<Meeting>, DbError>;

    /// Every tag on every meeting outside the trash, by meeting then tag.
    async fn links(&self) -> Result<Vec<MeetingTagLink>, DbError>;
}

#[async_trait]
impl MeetingTagRepo for SqliteTagRepo {
    async fn apply_tag(&self, application: &TagApplication) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "INSERT INTO meeting_tags (meeting_id, tag_id, applied_by, applied_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (meeting_id, tag_id) DO NOTHING",
            application.meeting_id,
            application.tag_id,
            application.applied_by,
            application.applied_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn remove_tag(&self, meeting: &MeetingId, tag: &TagId) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "DELETE FROM meeting_tags WHERE meeting_id = ?1 AND tag_id = ?2",
            meeting,
            tag,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn tags_of(&self, meeting: &MeetingId) -> Result<Vec<AppliedTag>, DbError> {
        let rows = sqlx::query_file_as!(AppliedTagRow, "src/tags/queries/tags_of.sql", meeting)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows.into_iter().map(AppliedTag::from).collect())
    }

    async fn meetings_tagged(&self, tag: &TagId) -> Result<Vec<Meeting>, DbError> {
        let meetings = sqlx::query_file_as!(Meeting, "src/tags/queries/meetings_tagged.sql", tag)
            .fetch_all(self.database.reader())
            .await?;
        Ok(meetings)
    }

    async fn links(&self) -> Result<Vec<MeetingTagLink>, DbError> {
        let links = sqlx::query_file_as!(MeetingTagLink, "src/tags/queries/links.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(links)
    }
}

/// One row of `tags_of.sql`: a tag joined with its application.
struct AppliedTagRow {
    id: TagId,
    name: String,
    color: PaletteColor,
    description: String,
    auto_apply: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    applied_by: TagSource,
    applied_at: DateTime<Utc>,
}

impl From<AppliedTagRow> for AppliedTag {
    fn from(row: AppliedTagRow) -> Self {
        Self {
            tag: Tag {
                id: row.id,
                name: row.name,
                color: row.color,
                description: row.description,
                auto_apply: row.auto_apply,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            applied_by: row.applied_by,
            applied_at: row.applied_at,
        }
    }
}
