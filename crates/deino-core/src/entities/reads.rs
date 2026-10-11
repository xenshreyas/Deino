//! The entity read port: the lists, a person's or company's meetings, and
//! what their summaries read. Its SQLite implementation computes every
//! aggregate on read (ADR 0030).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{CompanyId, Email, MeetingId, PersonId};

use super::{CompanyRow, EntityMeeting, PersonRow};
use crate::companies::CompanyDomain;

/// A meeting as an entity summary reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityDigest {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it happened.
    pub at: DateTime<Utc>,
    /// Its finished summary, in Markdown.
    pub summary: Option<String>,
    /// The user's own notes, as Readable Markdown.
    pub notes: Option<String>,
}

/// Reads people and companies with what the user did with them.
#[async_trait]
pub trait EntityReads: Send + Sync {
    /// Listed people (not merged, not the user), most recently met first;
    /// only those at `company` when one is given.
    async fn person_rows(&self, company: Option<&CompanyId>) -> Result<Vec<PersonRow>, DbError>;

    /// Listed companies, most recently met first.
    async fn company_rows(&self) -> Result<Vec<CompanyRow>, DbError>;

    /// Up to `limit` meetings the person attended, most recent first.
    async fn person_meetings(
        &self,
        person: &PersonId,
        limit: u32,
    ) -> Result<Vec<EntityMeeting>, DbError>;

    /// Up to `limit` meetings with the company's people, most recent first.
    async fn company_meetings(
        &self,
        company: &CompanyId,
        limit: u32,
    ) -> Result<Vec<EntityMeeting>, DbError>;

    /// Up to `limit` of the person's meetings with their summaries and notes.
    async fn person_digests(
        &self,
        person: &PersonId,
        limit: u32,
    ) -> Result<Vec<EntityDigest>, DbError>;

    /// Up to `limit` of the company's meetings with summaries and notes.
    async fn company_digests(
        &self,
        company: &CompanyId,
        limit: u32,
    ) -> Result<Vec<EntityDigest>, DbError>;
}

/// The entity ports over Deino's SQLite database: [`EntityReads`],
/// [`EntityWrites`](super::EntityWrites) and
/// [`EntityMerger`](super::EntityMerger).
#[derive(Clone, Debug)]
pub struct SqliteEntityRepo {
    pub(super) database: Database,
}

impl SqliteEntityRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl EntityReads for SqliteEntityRepo {
    async fn person_rows(&self, company: Option<&CompanyId>) -> Result<Vec<PersonRow>, DbError> {
        let rows = sqlx::query_file_as!(PersonRow, "src/entities/queries/person_rows.sql", company)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows)
    }

    async fn company_rows(&self) -> Result<Vec<CompanyRow>, DbError> {
        let rows = sqlx::query_file_as!(CompanyRow, "src/entities/queries/company_rows.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows)
    }

    async fn person_meetings(
        &self,
        person: &PersonId,
        limit: u32,
    ) -> Result<Vec<EntityMeeting>, DbError> {
        let meetings = sqlx::query_file_as!(
            EntityMeeting,
            "src/entities/queries/person_meetings.sql",
            person,
            limit
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(meetings)
    }

    async fn company_meetings(
        &self,
        company: &CompanyId,
        limit: u32,
    ) -> Result<Vec<EntityMeeting>, DbError> {
        let meetings = sqlx::query_file_as!(
            EntityMeeting,
            "src/entities/queries/company_meetings.sql",
            company,
            limit
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(meetings)
    }

    async fn person_digests(
        &self,
        person: &PersonId,
        limit: u32,
    ) -> Result<Vec<EntityDigest>, DbError> {
        let digests = sqlx::query_file_as!(
            EntityDigest,
            "src/entities/queries/person_digests.sql",
            person,
            limit
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(digests)
    }

    async fn company_digests(
        &self,
        company: &CompanyId,
        limit: u32,
    ) -> Result<Vec<EntityDigest>, DbError> {
        let digests = sqlx::query_file_as!(
            EntityDigest,
            "src/entities/queries/company_digests.sql",
            company,
            limit
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(digests)
    }
}

#[cfg(test)]
mod list_bench;
#[cfg(test)]
mod reads_tests;
