//! The calendar list port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{CalendarAccountId, CalendarId};

use super::{Calendar, CalendarColor, SqliteCalendarRepo};

/// Where the calendars of each account are read from. Only a sync writes
/// them ([`CalendarSyncRepo`](super::CalendarSyncRepo)).
#[async_trait]
pub trait CalendarRepo: Send + Sync {
    /// Every calendar, by account and title.
    async fn calendars(&self) -> Result<Vec<Calendar>, DbError>;
}

#[async_trait]
impl CalendarRepo for SqliteCalendarRepo {
    async fn calendars(&self) -> Result<Vec<Calendar>, DbError> {
        let calendars = sqlx::query_file_as!(Calendar, "src/calendar/queries/calendars.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(calendars)
    }
}
