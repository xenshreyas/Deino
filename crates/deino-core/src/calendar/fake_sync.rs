//! [`CalendarSyncRepo`] for the in-memory [`FakeCalendarRepo`], with the
//! SQLite repository's semantics (the contract tests run both).

use std::collections::BTreeSet;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{CalendarAccountId, CalendarId};

use super::batch::{SyncBatch, SyncReport, SyncedAccount, SyncedCalendar};
use super::fake::FakeCalendarState;
use super::{Calendar, CalendarAccount, CalendarSyncRepo, FakeCalendarRepo};

/// The stored account a batch account's rows go to.
struct Scope<'a> {
    batch: &'a SyncBatch,
    account_id: CalendarAccountId,
}

#[async_trait]
impl CalendarSyncRepo for FakeCalendarRepo {
    async fn apply(&self, batch: &SyncBatch) -> Result<SyncReport, DbError> {
        let mut state = self.state();
        let mut report = SyncReport::default();
        let names: BTreeSet<&str> = batch.accounts.iter().map(|a| a.name.as_str()).collect();
        let gone: Vec<CalendarAccountId> = state
            .accounts
            .iter()
            .filter(|account| account.provider == batch.provider)
            .filter(|account| !names.contains(account.name.as_str()))
            .map(|account| account.id)
            .collect();
        for id in gone {
            report.calendars_changed += usize::from(state.remove_account(id));
        }
        for account in &batch.accounts {
            let account_id = state.upsert_account(batch, account, &mut report);
            let scope = Scope { batch, account_id };
            state.sync_account(&scope, account, &mut report);
        }
        Ok(report)
    }
}

impl FakeCalendarState {
    fn upsert_account(
        &mut self,
        batch: &SyncBatch,
        account: &SyncedAccount,
        report: &mut SyncReport,
    ) -> CalendarAccountId {
        let stored = self
            .accounts
            .iter()
            .find(|stored| stored.provider == batch.provider && stored.name == account.name);
        if let Some(stored) = stored {
            return stored.id;
        }
        self.accounts.push(CalendarAccount {
            id: account.id,
            provider: batch.provider,
            name: account.name.clone(),
            created_at: batch.synced_at,
        });
        report.calendars_changed += 1;
        account.id
    }

    fn sync_account(
        &mut self,
        scope: &Scope<'_>,
        account: &SyncedAccount,
        report: &mut SyncReport,
    ) {
        let mut kept_calendars = BTreeSet::new();
        for calendar in &account.calendars {
            let calendar_id = self.upsert_calendar(scope, calendar, report);
            kept_calendars.insert(calendar_id);
            for event in &calendar.events {
                let row = event.to_row(scope.account_id, calendar_id, scope.batch.synced_at);
                report.count_upsert(event.id, self.upsert(&row));
            }
        }
        report.calendars_changed += self.remove_other_calendars(scope.account_id, &kept_calendars);
        let kept: BTreeSet<&str> = account
            .calendars
            .iter()
            .flat_map(|calendar| &calendar.events)
            .map(|event| event.provider_event_id.as_str())
            .collect();
        report.events_removed += self.remove_missing_events(scope, &kept);
    }

    fn upsert_calendar(
        &mut self,
        scope: &Scope<'_>,
        calendar: &SyncedCalendar,
        report: &mut SyncReport,
    ) -> CalendarId {
        let stored = self.calendars.iter_mut().find(|stored| {
            stored.account_id == scope.account_id
                && stored.provider_calendar_id == calendar.provider_calendar_id
        });
        if let Some(stored) = stored {
            if stored.title != calendar.title || stored.color != calendar.color {
                (stored.title, stored.color) = (calendar.title.clone(), calendar.color.clone());
                stored.updated_at = scope.batch.synced_at;
                report.calendars_changed += 1;
            }
            return stored.id;
        }
        self.calendars.push(Calendar {
            id: calendar.id,
            account_id: scope.account_id,
            provider_calendar_id: calendar.provider_calendar_id.clone(),
            title: calendar.title.clone(),
            color: calendar.color.clone(),
            created_at: scope.batch.synced_at,
            updated_at: scope.batch.synced_at,
        });
        report.calendars_changed += 1;
        calendar.id
    }

    /// Removes the account's calendars not in `kept`, with their events.
    fn remove_other_calendars(
        &mut self,
        account: CalendarAccountId,
        kept: &BTreeSet<CalendarId>,
    ) -> usize {
        let before = self.calendars.len();
        self.calendars
            .retain(|calendar| calendar.account_id != account || kept.contains(&calendar.id));
        let calendars: BTreeSet<CalendarId> =
            self.calendars.iter().map(|calendar| calendar.id).collect();
        self.events
            .retain(|event| event.calendar_id.is_none_or(|id| calendars.contains(&id)));
        before - self.calendars.len()
    }

    /// Removes the account's in-window events whose provider ID is not in `kept`.
    fn remove_missing_events(&mut self, scope: &Scope<'_>, kept: &BTreeSet<&str>) -> usize {
        let window = scope.batch.window;
        let before = self.events.len();
        self.events.retain(|event| {
            let in_window = event.start_at >= window.start && event.start_at < window.end;
            event.account_id != scope.account_id
                || !in_window
                || kept.contains(event.provider_event_id.as_str())
        });
        before - self.events.len()
    }
}
