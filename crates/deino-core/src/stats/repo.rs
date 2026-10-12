//! The stats read port and its SQLite adapter.
//!
//! Every read is one compile-checked query over the `stats_meetings` view
//! (the one definition of which meetings count) or `llm_calls`, on the read
//! pool. Nothing is stored: the service aggregates these rows on read.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{Email, MeetingId, PersonId};
use deino_llm::ProviderId;

/// A counted meeting: when it started and how long it was recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingFact {
    /// The meeting.
    pub id: MeetingId,
    /// When it started (or was saved, for old meetings).
    pub at: DateTime<Utc>,
    /// Its recorded length in seconds; `None` when unknown.
    pub duration_s: Option<i64>,
}

/// One person who took part in a counted meeting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendanceFact {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The person.
    pub person_id: PersonId,
    /// Their name.
    pub name: String,
    /// Their address, when known.
    pub email: Option<Email>,
    /// True for the user's own person.
    pub is_me: bool,
}

/// The calls to one provider and model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageFact {
    /// The provider.
    pub provider: ProviderId,
    /// The model ID.
    pub model: String,
    /// Calls made.
    pub calls: u32,
    /// Calls that failed.
    pub failed: u32,
    /// Prompt tokens.
    pub input_tokens: i64,
    /// Answer tokens.
    pub output_tokens: i64,
}

/// A counted meeting's title, for the year in review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleFact {
    /// The title.
    pub title: String,
    /// When the meeting started.
    pub at: DateTime<Utc>,
}

/// Reads the rows stats are computed from. `since: None` means all time.
#[async_trait]
pub trait StatsRepo: Send + Sync {
    /// The counted meetings that started at or after `since`.
    async fn meetings(&self, since: Option<DateTime<Utc>>) -> Result<Vec<MeetingFact>, DbError>;

    /// Who took part in those meetings, declined invitees left out.
    async fn attendance(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<AttendanceFact>, DbError>;

    /// LLM calls started at or after `since`, per provider and model.
    async fn llm_usage(&self, since: Option<DateTime<Utc>>) -> Result<Vec<UsageFact>, DbError>;

    /// Titles of the counted meetings that started in `[from, until)`,
    /// oldest first, at most `limit`.
    async fn titles(
        &self,
        (from, until): (DateTime<Utc>, DateTime<Utc>),
        limit: u32,
    ) -> Result<Vec<TitleFact>, DbError>;
}

/// [`StatsRepo`] over SQLite.
#[derive(Clone, Debug)]
pub struct SqliteStatsRepo {
    database: Database,
}

impl SqliteStatsRepo {
    /// A repository over the app database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl StatsRepo for SqliteStatsRepo {
    async fn meetings(&self, since: Option<DateTime<Utc>>) -> Result<Vec<MeetingFact>, DbError> {
        let rows = sqlx::query_file_as!(MeetingFact, "src/stats/queries/meetings.sql", since)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows)
    }

    async fn attendance(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<AttendanceFact>, DbError> {
        let rows = sqlx::query_file_as!(AttendanceFact, "src/stats/queries/attendance.sql", since)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows)
    }

    async fn llm_usage(&self, since: Option<DateTime<Utc>>) -> Result<Vec<UsageFact>, DbError> {
        let rows = sqlx::query_file_as!(UsageFact, "src/stats/queries/llm_usage.sql", since)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows)
    }

    async fn titles(
        &self,
        (from, until): (DateTime<Utc>, DateTime<Utc>),
        limit: u32,
    ) -> Result<Vec<TitleFact>, DbError> {
        let rows = sqlx::query_file_as!(
            TitleFact,
            "src/stats/queries/titles.sql",
            from,
            until,
            limit
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows)
    }
}
