//! The related-meetings port and its SQLite implementation: what a meeting
//! is related by, and the meetings that share its series or its title.

use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{MeetingId, PersonId, SeriesId};

/// The most candidates one lookup reads; a daily stand-up has a few hundred
/// meetings, of which only the newest matter.
const MAX_CANDIDATES: i64 = 100;

/// What a meeting is related to others by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationKey {
    /// The meeting.
    pub meeting: MeetingId,
    /// Its title.
    pub title: String,
    /// Its recurring series.
    pub series_id: Option<SeriesId>,
    /// When it took place: recording start, else creation.
    pub happened_at: DateTime<Utc>,
    /// Its attendees.
    pub attendees: BTreeSet<PersonId>,
}

/// A meeting that shares the key's series or title, not yet judged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelatedCandidate {
    /// The meeting.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// Its recurring series.
    pub series_id: Option<SeriesId>,
    /// When it took place.
    pub happened_at: DateTime<Utc>,
    /// Its attendees.
    pub attendees: BTreeSet<PersonId>,
}

/// Reads what related meetings are found from.
#[async_trait]
pub trait RelatedMeetingRepo: Send + Sync {
    /// The key of `meeting`; `None` when no meeting has the ID.
    async fn key(&self, meeting: &MeetingId) -> Result<Option<RelationKey>, DbError>;

    /// The meetings outside the trash, other than the key's, in its series
    /// or titled exactly like it, newest first.
    async fn candidates(&self, key: &RelationKey) -> Result<Vec<RelatedCandidate>, DbError>;
}

/// [`RelatedMeetingRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteRelatedRepo {
    database: Database,
}

impl SqliteRelatedRepo {
    /// A repository over `database`.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl RelatedMeetingRepo for SqliteRelatedRepo {
    async fn key(&self, meeting: &MeetingId) -> Result<Option<RelationKey>, DbError> {
        let row =
            sqlx::query_file_as!(Row, "src/calendar/linking/queries/related_key.sql", meeting)
                .fetch_optional(self.database.reader())
                .await?;
        Ok(row.map(|row| RelationKey {
            meeting: row.id,
            title: row.title,
            series_id: row.series_id,
            happened_at: row.happened_at,
            attendees: people(row.attendees.as_deref()),
        }))
    }

    async fn candidates(&self, key: &RelationKey) -> Result<Vec<RelatedCandidate>, DbError> {
        let rows = sqlx::query_file_as!(
            Row,
            "src/calendar/linking/queries/related_candidates.sql",
            key.meeting,
            key.series_id,
            key.title,
            MAX_CANDIDATES,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(RelatedCandidate::from).collect())
    }
}

/// One meeting row with its attendees, as both queries read it.
struct Row {
    id: MeetingId,
    title: String,
    series_id: Option<SeriesId>,
    happened_at: DateTime<Utc>,
    /// The attendees' IDs, space-separated, or `None` without attendees.
    attendees: Option<String>,
}

impl From<Row> for RelatedCandidate {
    fn from(row: Row) -> Self {
        Self {
            attendees: people(row.attendees.as_deref()),
            id: row.id,
            title: row.title,
            series_id: row.series_id,
            happened_at: row.happened_at,
        }
    }
}

/// The person IDs in a space-separated list; anything unparsable is
/// skipped (IDs are written by Deino, so there is none).
fn people(list: Option<&str>) -> BTreeSet<PersonId> {
    list.unwrap_or_default()
        .split_whitespace()
        .filter_map(|id| id.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attendee_lists_parse_into_sets() {
        let one = "01926b2c-0000-7000-8000-000000000001";
        let two = "01926b2c-0000-7000-8000-000000000002";

        let parsed = people(Some(&format!("{two} {one} {two}")));

        assert_eq!(parsed.len(), 2);
        assert!(people(None).is_empty());
        assert!(people(Some("")).is_empty());
    }
}
