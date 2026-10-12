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

    /// Records an imported meeting.
    async fn record(&self, entry: &LedgerEntry) -> Result<(), DbError>;
}

/// A source meeting and the meeting it became.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerEntry {
    /// Where it came from.
    pub source: ImportSource,
    /// Its ID there.
    pub source_id: String,
    /// The meeting it became.
    pub meeting: MeetingId,
    /// When it was imported.
    pub at: DateTime<Utc>,
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

    async fn record(&self, entry: &LedgerEntry) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO imported_meetings (source, source_id, meeting_id, imported_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (source, source_id) DO UPDATE SET meeting_id = excluded.meeting_id, \
             imported_at = excluded.imported_at",
            entry.source,
            entry.source_id,
            entry.meeting,
            entry.at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }
}
