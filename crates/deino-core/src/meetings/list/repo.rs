//! The list port and its SQLite implementation: one query for a page of
//! rows, then one each for the page's avatars and tag chips, so a page is
//! always three statements however many rows it has.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{MeetingId, MeetingStatus, PaletteColor, PersonId, Seconds, TagId};
use sqlx::types::Json;

use super::{AttendeeRow, ListQuery, ListedAttendee, ListedTag, MeetingCursor, MeetingRow, TagRow};
use crate::meetings::SqliteMeetingRepo;

/// How many rows a page query returns: a bound, or every match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowLimit {
    /// At most this many.
    At(u32),
    /// Every match (⌘A selects all matching meetings).
    All,
}

impl RowLimit {
    /// The SQL `LIMIT`: SQLite reads a negative one as "no limit".
    fn sql(self) -> i64 {
        match self {
            Self::At(rows) => i64::from(rows),
            Self::All => -1,
        }
    }
}

/// Reads the Meetings page.
#[async_trait]
pub trait MeetingListRepo: Send + Sync {
    /// Meetings outside the trash that `query` matches, newest first,
    /// starting after `after`.
    async fn page(
        &self,
        query: &ListQuery,
        after: Option<&MeetingCursor>,
        limit: RowLimit,
    ) -> Result<Vec<MeetingRow>, DbError>;

    /// The attendees of `meetings`, grouped by meeting, the user last.
    async fn attendees(&self, meetings: &[MeetingId]) -> Result<Vec<AttendeeRow>, DbError>;

    /// The tags on `meetings`, grouped by meeting, by name.
    async fn tags(&self, meetings: &[MeetingId]) -> Result<Vec<TagRow>, DbError>;
}

/// One `attendees.sql` row.
struct AttendeeSqlRow {
    meeting_id: MeetingId,
    person_id: PersonId,
    name: String,
    avatar_path: Option<String>,
    is_me: bool,
}

/// One `tags.sql` row.
struct TagSqlRow {
    meeting_id: MeetingId,
    id: TagId,
    name: String,
    color: PaletteColor,
}

#[async_trait]
impl MeetingListRepo for SqliteMeetingRepo {
    async fn page(
        &self,
        query: &ListQuery,
        after: Option<&MeetingCursor>,
        limit: RowLimit,
    ) -> Result<Vec<MeetingRow>, DbError> {
        let audience = &query.audience;
        let (cursor_at, cursor_id) = (after.map(|c| c.at.as_str()), after.map(|c| &c.id));
        let (people, companies) = (Json(&query.people), Json(&query.companies));
        let (tags, folders) = (Json(&query.tags), Json(&query.folders));
        let (own, domains) = (Json(&audience.own), Json(&audience.domains));
        let limit = limit.sql();
        let rows = sqlx::query_file_as!(
            MeetingRow,
            "src/meetings/list/queries/page.sql",
            query.held_from,
            query.held_before,
            people,
            companies,
            tags,
            folders,
            query.has_action_items,
            audience.audience,
            own,
            domains,
            cursor_at,
            cursor_id,
            limit,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows)
    }

    async fn attendees(&self, meetings: &[MeetingId]) -> Result<Vec<AttendeeRow>, DbError> {
        let meetings = Json(meetings);
        let rows = sqlx::query_file_as!(
            AttendeeSqlRow,
            "src/meetings/list/queries/attendees.sql",
            meetings
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(AttendeeSqlRow::into_row).collect())
    }

    async fn tags(&self, meetings: &[MeetingId]) -> Result<Vec<TagRow>, DbError> {
        let meetings = Json(meetings);
        let rows = sqlx::query_file_as!(TagSqlRow, "src/meetings/list/queries/tags.sql", meetings)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows.into_iter().map(TagSqlRow::into_row).collect())
    }
}

impl AttendeeSqlRow {
    fn into_row(self) -> AttendeeRow {
        AttendeeRow {
            meeting_id: self.meeting_id,
            attendee: ListedAttendee {
                person_id: self.person_id,
                name: self.name,
                avatar_path: self.avatar_path,
                is_me: self.is_me,
            },
        }
    }
}

impl TagSqlRow {
    fn into_row(self) -> TagRow {
        TagRow {
            meeting_id: self.meeting_id,
            tag: ListedTag {
                id: self.id,
                name: self.name,
                color: self.color,
            },
        }
    }
}
