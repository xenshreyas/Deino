//! An in-memory store for the calendar ports.

use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{CalendarAccountId, CalendarEventId};

use super::{
    Calendar, CalendarAccount, CalendarAccountRepo, CalendarEvent, CalendarEventRepo, CalendarRepo,
};
use crate::testing::lock;

/// Keeps accounts, calendars and events in `Vec`s and honours the
/// [`CalendarAccountRepo`], [`CalendarRepo`], [`CalendarEventRepo`] and
/// [`CalendarSyncRepo`](super::CalendarSyncRepo) contracts: upserts keep the
/// stored ID, `created_at` and override and write only real changes,
/// removing an account or calendar removes its events, and lists keep their
/// orders.
#[derive(Debug, Default)]
pub struct FakeCalendarRepo {
    pub(super) state: Mutex<FakeCalendarState>,
}

/// The rows a [`FakeCalendarRepo`] holds.
#[derive(Debug, Default)]
pub(super) struct FakeCalendarState {
    pub(super) accounts: Vec<CalendarAccount>,
    pub(super) calendars: Vec<Calendar>,
    pub(super) events: Vec<CalendarEvent>,
}

impl FakeCalendarRepo {
    pub(super) fn state(&self) -> MutexGuard<'_, FakeCalendarState> {
        lock(&self.state)
    }
}

impl FakeCalendarState {
    /// Removes an account with its calendars and events.
    pub(super) fn remove_account(&mut self, id: CalendarAccountId) -> bool {
        let before = self.accounts.len();
        self.accounts.retain(|account| account.id != id);
        self.calendars.retain(|calendar| calendar.account_id != id);
        self.events.retain(|event| event.account_id != id);
        self.accounts.len() < before
    }

    /// Stores or refreshes `event` like the SQL upsert; returns the stored ID
    /// when something was written.
    pub(super) fn upsert(&mut self, event: &CalendarEvent) -> Option<CalendarEventId> {
        let existing = self.events.iter_mut().find(|stored| {
            stored.account_id == event.account_id
                && stored.provider_event_id == event.provider_event_id
        });
        let Some(stored) = existing else {
            self.events.push(event.clone());
            return Some(event.id);
        };
        let mut refreshed = event.clone();
        (
            refreshed.id,
            refreshed.created_at,
            refreshed.record_override,
        ) = (stored.id, stored.created_at, stored.record_override);
        refreshed.updated_at = stored.updated_at;
        if refreshed == *stored {
            return None;
        }
        refreshed.updated_at = event.updated_at;
        *stored = refreshed;
        Some(stored.id)
    }
}

#[async_trait]
impl CalendarAccountRepo for FakeCalendarRepo {
    async fn add_account(&self, account: &CalendarAccount) -> Result<(), DbError> {
        self.state().accounts.push(account.clone());
        Ok(())
    }

    async fn accounts(&self) -> Result<Vec<CalendarAccount>, DbError> {
        let mut accounts = self.state().accounts.clone();
        accounts.sort_by(|a, b| (a.provider, &a.name).cmp(&(b.provider, &b.name)));
        Ok(accounts)
    }

    async fn remove_account(&self, id: &CalendarAccountId) -> Result<bool, DbError> {
        Ok(self.state().remove_account(*id))
    }
}

#[async_trait]
impl CalendarRepo for FakeCalendarRepo {
    async fn calendars(&self) -> Result<Vec<Calendar>, DbError> {
        let mut calendars = self.state().calendars.clone();
        calendars.sort_by(|a, b| {
            let key = |calendar: &Calendar| {
                (
                    calendar.account_id,
                    calendar.title.to_lowercase(),
                    calendar.id,
                )
            };
            key(a).cmp(&key(b))
        });
        Ok(calendars)
    }
}

#[async_trait]
impl CalendarEventRepo for FakeCalendarRepo {
    async fn upsert_event(&self, event: &CalendarEvent) -> Result<CalendarEventId, DbError> {
        let mut state = self.state();
        if let Some(id) = state.upsert(event) {
            return Ok(id);
        }
        let stored = state.events.iter().find(|stored| {
            stored.account_id == event.account_id
                && stored.provider_event_id == event.provider_event_id
        });
        Ok(stored.map_or(event.id, |stored| stored.id))
    }

    async fn event(&self, id: &CalendarEventId) -> Result<Option<CalendarEvent>, DbError> {
        Ok(self
            .state()
            .events
            .iter()
            .find(|event| event.id == *id)
            .cloned())
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, DbError> {
        let state = self.state();
        let mut listed: Vec<CalendarEvent> = state
            .events
            .iter()
            .filter(|event| event.start_at >= from && event.start_at < to)
            .cloned()
            .collect();
        listed.sort_by_key(|event| (event.start_at, event.id));
        Ok(listed)
    }

    async fn set_record_override(
        &self,
        id: &CalendarEventId,
        record: Option<bool>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let mut state = self.state();
        let Some(event) = state.events.iter_mut().find(|event| event.id == *id) else {
            return Ok(false);
        };
        event.record_override = record;
        event.updated_at = at;
        Ok(true)
    }

    async fn delete_event(&self, id: &CalendarEventId) -> Result<bool, DbError> {
        let mut state = self.state();
        let before = state.events.len();
        state.events.retain(|event| event.id != *id);
        Ok(state.events.len() < before)
    }
}
