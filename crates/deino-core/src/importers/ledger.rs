//! The de-duplication ledger (`imported_meetings`): which source meetings
//! were imported, so importing twice adds nothing. Rows outlive their
//! meeting, so a meeting the user deleted is never brought back.

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{ImportSource, MeetingId};

/// Where imported source IDs are kept.
#[async_trait]
pub trait ImportLedger: Send + Sync {
    /// Every ID imported from `source`.
    async fn imported(&self, source: ImportSource) -> Result<HashSet<String>, DbError>;

    /// Records that `source_id` became `meeting` at `at`.
    async fn record(
        &self,
        source: ImportSource,
        source_id: &str,
        meeting: &MeetingId,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;
}

/// [`ImportLedger`] over SQLite.
#[derive(Clone, Debug)]
pub struct SqliteImportLedger {
    database: Database,
}

impl SqliteImportLedger {
    /// The ledger in the app's database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ImportLedger for SqliteImportLedger {
    async fn imported(&self, source: ImportSource) -> Result<HashSet<String>, DbError> {
        let ids = sqlx::query_scalar!(
            "SELECT source_id FROM imported_meetings WHERE source = ?1",
            source
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(ids.into_iter().collect())
    }

    async fn record(
        &self,
        source: ImportSource,
        source_id: &str,
        meeting: &MeetingId,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO imported_meetings (source, source_id, meeting_id, imported_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (source, source_id) DO UPDATE SET meeting_id = excluded.meeting_id, \
             imported_at = excluded.imported_at",
            source,
            source_id,
            meeting,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }
}
