//! The sync port: storing one provider's [`SyncBatch`] in one transaction.
//!
//! A batch is the provider's whole truth for its window, so storing it
//! upserts every account, calendar and event in it and deletes what the
//! provider no longer has: other accounts of the provider, other calendars
//! of each account, and stored events of each account starting inside the
//! window that the batch lacks (cancelled, deleted, moved away). Rows are
//! touched only when something changed, so a re-sync of an unchanged
//! calendar commits nothing and the UI hears nothing.

use std::collections::BTreeSet;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{CalendarAccountId, CalendarEventId, CalendarId, CalendarProvider};
use sqlx::SqliteConnection;

use super::SqliteCalendarRepo;
use super::batch::{SyncBatch, SyncReport, SyncedAccount, SyncedCalendar};
use super::event_repo::upsert_on;

/// Stores what a sync read.
#[async_trait]
pub trait CalendarSyncRepo: Send + Sync {
    /// Makes the stored accounts, calendars and in-window events of
    /// `batch.provider` equal to `batch`, keeping stored IDs, `created_at`
    /// and the user's record overrides. One transaction.
    async fn apply(&self, batch: &SyncBatch) -> Result<SyncReport, DbError>;
}

#[async_trait]
impl CalendarSyncRepo for SqliteCalendarRepo {
    async fn apply(&self, batch: &SyncBatch) -> Result<SyncReport, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let mut report = SyncReport::default();
        let names: BTreeSet<&str> = batch.accounts.iter().map(|a| a.name.as_str()).collect();
        report.calendars_changed +=
            remove_other_accounts(&mut transaction, batch.provider, &names).await?;
        for account in &batch.accounts {
            let account_id = upsert_account(&mut transaction, batch, account, &mut report).await?;
            let target = AccountTarget { batch, account_id };
            sync_account(&mut transaction, &target, account, &mut report).await?;
        }
        transaction.commit().await?;
        Ok(report)
    }
}

/// The stored account a batch account's rows go to.
struct AccountTarget<'a> {
    batch: &'a SyncBatch,
    account_id: CalendarAccountId,
}

/// Deletes the provider's accounts that are not in `names` (with their
/// calendars and events); returns how many.
async fn remove_other_accounts(
    connection: &mut SqliteConnection,
    provider: CalendarProvider,
    names: &BTreeSet<&str>,
) -> Result<usize, DbError> {
    let stored = sqlx::query!(
        r#"SELECT id AS "id: CalendarAccountId", name FROM calendar_accounts WHERE provider = ?1"#,
        provider,
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut removed = 0;
    for account in stored
        .iter()
        .filter(|row| !names.contains(row.name.as_str()))
    {
        sqlx::query!("DELETE FROM calendar_accounts WHERE id = ?1", account.id)
            .execute(&mut *connection)
            .await?;
        removed += 1;
    }
    Ok(removed)
}

/// Stores the account if it is new; returns its stored ID.
async fn upsert_account(
    connection: &mut SqliteConnection,
    batch: &SyncBatch,
    account: &SyncedAccount,
    report: &mut SyncReport,
) -> Result<CalendarAccountId, DbError> {
    let inserted = sqlx::query!(
        "INSERT INTO calendar_accounts (id, provider, name, created_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (provider, name) DO NOTHING",
        account.id,
        batch.provider,
        account.name,
        batch.synced_at,
    )
    .execute(&mut *connection)
    .await?;
    report.calendars_changed += usize::from(inserted.rows_affected() > 0);
    let id = sqlx::query_scalar!(
        r#"SELECT id AS "id: CalendarAccountId" FROM calendar_accounts WHERE provider = ?1 AND name = ?2"#,
        batch.provider,
        account.name,
    )
    .fetch_one(&mut *connection)
    .await?;
    Ok(id)
}

/// Stores one account's calendars and events and removes what it lost.
async fn sync_account(
    connection: &mut SqliteConnection,
    target: &AccountTarget<'_>,
    account: &SyncedAccount,
    report: &mut SyncReport,
) -> Result<(), DbError> {
    let mut kept_calendars = BTreeSet::new();
    for calendar in &account.calendars {
        let calendar_id = upsert_calendar(connection, target, calendar, report).await?;
        kept_calendars.insert(calendar_id);
        for event in &calendar.events {
            let row = event.to_row(target.account_id, calendar_id, target.batch.synced_at);
            report.count_upsert(event.id, upsert_on(connection, &row).await?);
        }
    }
    report.calendars_changed +=
        remove_other_calendars(connection, target.account_id, &kept_calendars).await?;
    let events = account
        .calendars
        .iter()
        .flat_map(|calendar| &calendar.events);
    let kept_events: BTreeSet<&str> = events
        .map(|event| event.provider_event_id.as_str())
        .collect();
    report.events_removed += remove_missing_events(connection, target, &kept_events).await?;
    Ok(())
}

/// Stores the calendar, or refreshes its title and colour; returns its stored ID.
async fn upsert_calendar(
    connection: &mut SqliteConnection,
    target: &AccountTarget<'_>,
    calendar: &SyncedCalendar,
    report: &mut SyncReport,
) -> Result<CalendarId, DbError> {
    let written = sqlx::query!(
        "INSERT INTO calendars (id, account_id, provider_calendar_id, title, color, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
         ON CONFLICT (account_id, provider_calendar_id) DO UPDATE SET
             title = excluded.title, color = excluded.color, updated_at = excluded.updated_at
         WHERE calendars.title IS NOT excluded.title OR calendars.color IS NOT excluded.color",
        calendar.id,
        target.account_id,
        calendar.provider_calendar_id,
        calendar.title,
        calendar.color,
        target.batch.synced_at,
    )
    .execute(&mut *connection)
    .await?;
    report.calendars_changed += usize::from(written.rows_affected() > 0);
    let id = sqlx::query_scalar!(
        r#"SELECT id AS "id: CalendarId" FROM calendars WHERE account_id = ?1 AND provider_calendar_id = ?2"#,
        target.account_id,
        calendar.provider_calendar_id,
    )
    .fetch_one(&mut *connection)
    .await?;
    Ok(id)
}

/// Deletes the account's calendars not in `kept` (with their events).
async fn remove_other_calendars(
    connection: &mut SqliteConnection,
    account: CalendarAccountId,
    kept: &BTreeSet<CalendarId>,
) -> Result<usize, DbError> {
    let stored = sqlx::query_scalar!(
        r#"SELECT id AS "id: CalendarId" FROM calendars WHERE account_id = ?1"#,
        account,
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut removed = 0;
    for id in stored.iter().filter(|id| !kept.contains(id)) {
        sqlx::query!("DELETE FROM calendars WHERE id = ?1", id)
            .execute(&mut *connection)
            .await?;
        removed += 1;
    }
    Ok(removed)
}

/// Deletes the account's events starting inside the window whose provider
/// ID is not in `kept`; returns how many.
async fn remove_missing_events(
    connection: &mut SqliteConnection,
    target: &AccountTarget<'_>,
    kept: &BTreeSet<&str>,
) -> Result<usize, DbError> {
    let window = target.batch.window;
    let stored = sqlx::query!(
        r#"SELECT id AS "id: CalendarEventId", provider_event_id FROM calendar_events
        WHERE account_id = ?1 AND start_at >= ?2 AND start_at < ?3"#,
        target.account_id,
        window.start,
        window.end,
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut removed = 0;
    for event in stored
        .iter()
        .filter(|row| !kept.contains(row.provider_event_id.as_str()))
    {
        sqlx::query!("DELETE FROM calendar_events WHERE id = ?1", event.id)
            .execute(&mut *connection)
            .await?;
        removed += 1;
    }
    Ok(removed)
}
