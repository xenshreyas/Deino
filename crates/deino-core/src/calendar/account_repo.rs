//! The calendar account port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{CalendarAccountId, CalendarProvider};

use super::CalendarAccount;

/// Where connected calendar accounts are stored.
#[async_trait]
pub trait CalendarAccountRepo: Send + Sync {
    /// Connects an account. Fails if the provider already has one by that name.
    async fn add_account(&self, account: &CalendarAccount) -> Result<(), DbError>;

    /// Every account, by provider and name.
    async fn accounts(&self) -> Result<Vec<CalendarAccount>, DbError>;

    /// Disconnects the account and deletes its events.
    async fn remove_account(&self, id: &CalendarAccountId) -> Result<bool, DbError>;
}

/// [`CalendarAccountRepo`] and [`CalendarEventRepo`](super::CalendarEventRepo)
/// over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteCalendarRepo {
    pub(super) database: Database,
}

impl SqliteCalendarRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl CalendarAccountRepo for SqliteCalendarRepo {
    async fn add_account(&self, account: &CalendarAccount) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO calendar_accounts (id, provider, name, created_at) VALUES (?1, ?2, ?3, ?4)",
            account.id,
            account.provider,
            account.name,
            account.created_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn accounts(&self) -> Result<Vec<CalendarAccount>, DbError> {
        let accounts = sqlx::query_file_as!(CalendarAccount, "src/calendar/queries/accounts.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(accounts)
    }

    async fn remove_account(&self, id: &CalendarAccountId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM calendar_accounts WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
