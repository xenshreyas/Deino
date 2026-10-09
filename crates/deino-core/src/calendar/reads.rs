//! What the UI lists: calendars with their visibility, "Coming up", one
//! event, and the synchronous lookup meeting detection uses.

use std::collections::BTreeMap;
use std::sync::PoisonError;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{CalendarEventId, CalendarId};

use super::error::MAX_RANGE_DAYS;
use super::filter::{TimeRange, agenda};
use super::{CalendarAccount, CalendarError, CalendarEvent, CalendarService, CalendarView};

/// How far before a range's start an event may begin and still be listed
/// as ongoing (long all-day blocks aside, meetings are shorter).
const LONGEST_ONGOING: TimeDelta = TimeDelta::days(1);

/// The span around now kept in memory for [`CalendarService::events_near`].
const NEARBY_BEFORE: TimeDelta = TimeDelta::hours(1);
/// See [`NEARBY_BEFORE`].
const NEARBY_AFTER: TimeDelta = TimeDelta::hours(2);

impl CalendarService {
    /// Every calendar with its account and visibility, by account.
    pub async fn calendars(&self) -> Result<Vec<CalendarView>, CalendarError> {
        let accounts: BTreeMap<_, CalendarAccount> = self
            .store
            .accounts
            .accounts()
            .await?
            .into_iter()
            .map(|account| (account.id, account))
            .collect();
        let settings = self.settings.get().await.calendar;
        let calendars = self.store.calendars.calendars().await?;
        let mut views: Vec<CalendarView> = calendars
            .into_iter()
            .filter_map(|calendar| {
                let account = accounts.get(&calendar.account_id)?;
                Some(CalendarView {
                    visible: settings.is_visible(&calendar.id),
                    id: calendar.id,
                    title: calendar.title,
                    color: calendar.color,
                    account_id: account.id,
                    account_name: account.name.clone(),
                    provider: account.provider,
                })
            })
            .collect();
        views.sort_by(|a, b| (a.provider, &a.account_name).cmp(&(b.provider, &b.account_name)));
        Ok(views)
    }

    /// Shows or hides a calendar's events and returns it.
    pub async fn set_calendar_visible(
        &self,
        id: CalendarId,
        visible: bool,
    ) -> Result<CalendarView, CalendarError> {
        let calendars = self.calendars().await?;
        let mut view = calendars
            .into_iter()
            .find(|calendar| calendar.id == id)
            .ok_or(CalendarError::CalendarNotFound(id))?;
        self.settings
            .update(|settings| settings.calendar.set_visible(id, visible))
            .await?;
        view.visible = visible;
        self.refresh_nearby().await;
        self.announce().await;
        Ok(view)
    }

    /// The visible events overlapping `range`, by start time; empty while
    /// sync is off.
    pub async fn upcoming(&self, range: TimeRange) -> Result<Vec<CalendarEvent>, CalendarError> {
        let span = range.end - range.start;
        if span <= TimeDelta::zero() || span > TimeDelta::days(MAX_RANGE_DAYS) {
            return Err(CalendarError::InvalidRange);
        }
        let settings = self.settings.get().await.calendar;
        if !settings.sync_enabled {
            return Ok(Vec::new());
        }
        let stored = self
            .store
            .events
            .events_between(range.start - LONGEST_ONGOING, range.end)
            .await?;
        Ok(agenda(stored, &settings.hidden_calendars, &range))
    }

    /// One stored event.
    pub async fn event(&self, id: CalendarEventId) -> Result<CalendarEvent, CalendarError> {
        self.store
            .events
            .event(&id)
            .await?
            .ok_or(CalendarError::EventNotFound(id))
    }

    /// The visible events starting within an hour before or two hours after
    /// the last refresh, without touching the database: for meeting
    /// detection (CAP-06's `CalendarLookup`), which asks synchronously.
    pub fn events_near(&self, now: DateTime<Utc>) -> Vec<CalendarEvent> {
        let nearby = self.nearby.read().unwrap_or_else(PoisonError::into_inner);
        nearby
            .iter()
            .filter(|event| (event.start_at - now).abs() <= NEARBY_AFTER)
            .cloned()
            .collect()
    }

    /// Reloads the in-memory events around now.
    pub(super) async fn refresh_nearby(&self) {
        let now = self.clock.now();
        let range = TimeRange {
            start: now - NEARBY_BEFORE,
            end: now + NEARBY_AFTER,
        };
        match self.upcoming(range).await {
            Ok(events) => self.write_nearby(events),
            Err(error) => {
                log::warn!(target: "calendar", "could not load the events around now: {error}");
            }
        }
    }

    pub(super) fn write_nearby(&self, events: Vec<CalendarEvent>) {
        *self.nearby.write().unwrap_or_else(PoisonError::into_inner) = events;
    }
}
