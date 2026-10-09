//! An in-memory store for the calendar ports.

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{CalendarAccountId, CalendarEventId};

use super::{CalendarAccount, CalendarAccountRepo, CalendarEvent, CalendarEventRepo};
use crate::testing::lock;

/// Keeps accounts and events in `Vec`s and honours the
/// [`CalendarAccountRepo`] and [`CalendarEventRepo`] contracts: upserts keep
/// the stored ID, `created_at` and override, removing an account removes
/// its events, and lists keep their orders.
#[derive(Debug, Default)]
pub struct FakeCalendarRepo {
    accounts: Mutex<Vec<CalendarAccount>>,
    events: Mutex<Vec<CalendarEvent>>,
}

#[async_trait]
impl CalendarAccountRepo for FakeCalendarRepo {
    async fn add_account(&self, account: &CalendarAccount) -> Result<(), DbError> {
        lock(&self.accounts).push(account.clone());
        Ok(())
    }

    async fn accounts(&self) -> Result<Vec<CalendarAccount>, DbError> {
        let mut accounts = lock(&self.accounts).clone();
        accounts.sort_by(|a, b| (a.provider, &a.name).cmp(&(b.provider, &b.name)));
        Ok(accounts)
    }

    async fn remove_account(&self, id: &CalendarAccountId) -> Result<bool, DbError> {
        let mut accounts = lock(&self.accounts);
        let before = accounts.len();
        accounts.retain(|account| account.id != *id);
        lock(&self.events).retain(|event| event.account_id != *id);
        Ok(accounts.len() < before)
    }
}

#[async_trait]
impl CalendarEventRepo for FakeCalendarRepo {
    async fn upsert_event(&self, event: &CalendarEvent) -> Result<CalendarEventId, DbError> {
        let mut events = lock(&self.events);
        let existing = events.iter_mut().find(|stored| {
            stored.account_id == event.account_id
                && stored.provider_event_id == event.provider_event_id
        });
        let Some(stored) = existing else {
            events.push(event.clone());
            return Ok(event.id);
        };
        let kept = (stored.id, stored.created_at, stored.record_override);
        *stored = event.clone();
        (stored.id, stored.created_at, stored.record_override) = kept;
        Ok(stored.id)
    }

    async fn event(&self, id: &CalendarEventId) -> Result<Option<CalendarEvent>, DbError> {
        Ok(lock(&self.events)
            .iter()
            .find(|event| event.id == *id)
            .cloned())
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, DbError> {
        let events = lock(&self.events);
        let mut listed: Vec<CalendarEvent> = events
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
        let mut events = lock(&self.events);
        let Some(event) = events.iter_mut().find(|event| event.id == *id) else {
            return Ok(false);
        };
        event.record_override = record;
        event.updated_at = at;
        Ok(true)
    }

    async fn delete_event(&self, id: &CalendarEventId) -> Result<bool, DbError> {
        let mut events = lock(&self.events);
        let before = events.len();
        events.retain(|event| event.id != *id);
        Ok(events.len() < before)
    }
}
