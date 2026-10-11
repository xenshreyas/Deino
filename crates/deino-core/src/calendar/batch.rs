//! A sync's result, ready to store: [`SyncBatch`] and what storing it changed.
//!
//! Every ID in a batch is a candidate: the repository keeps the stored ID of
//! an account, calendar or event it already has (matched by provider name or
//! provider ID) and uses the candidate only for new rows.

use chrono::{DateTime, Utc};
use deino_domain::{
    CalendarAccountId, CalendarEventId, CalendarId, CalendarProvider, ConferencePlatform, Rsvp,
    SeriesId,
};

use super::source::SyncWindow;
use super::{CalendarColor, CalendarEvent, EventAttendee};

/// One provider's accounts, calendars and events in one window.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncBatch {
    /// The provider read.
    pub provider: CalendarProvider,
    /// The span read; stored events of these accounts starting inside it
    /// and missing from the batch were deleted or moved away.
    pub window: SyncWindow,
    /// When the provider was read.
    pub synced_at: DateTime<Utc>,
    /// Every account the provider has now; others of the provider are gone.
    pub accounts: Vec<SyncedAccount>,
}

/// One account and all its calendars.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncedAccount {
    /// A candidate ID for a new account.
    pub id: CalendarAccountId,
    /// The provider's name for it; unique per provider.
    pub name: String,
    /// Every calendar it has now; others of the account are gone.
    pub calendars: Vec<SyncedCalendar>,
}

/// One calendar and its events in the window.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncedCalendar {
    /// A candidate ID for a new calendar.
    pub id: CalendarId,
    /// The provider's ID for it.
    pub provider_calendar_id: String,
    /// Its name.
    pub title: String,
    /// Its colour.
    pub color: Option<CalendarColor>,
    /// Its events, already filtered (see `filter::keeps`).
    pub events: Vec<SyncedEvent>,
}

/// One event as the provider has it, with the core's reading of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncedEvent {
    /// A candidate ID for a new event.
    pub id: CalendarEventId,
    /// The provider's ID for this occurrence.
    pub provider_event_id: String,
    /// The iCalendar UID.
    pub ical_uid: Option<String>,
    /// The recurring series it belongs to.
    pub series_id: Option<SeriesId>,
    /// Its title.
    pub title: String,
    /// Its description.
    pub description: String,
    /// When it starts.
    pub start_at: DateTime<Utc>,
    /// When it ends; never before `start_at`.
    pub end_at: DateTime<Utc>,
    /// True for an all-day event.
    pub is_all_day: bool,
    /// The join link.
    pub conference_url: Option<String>,
    /// The service of the join link.
    pub platform: Option<ConferencePlatform>,
    /// Who organised it.
    pub organizer_email: Option<String>,
    /// The people invited (no rooms or resources).
    pub attendees: Vec<EventAttendee>,
    /// The user's answer.
    pub my_rsvp: Option<Rsvp>,
    /// True when someone outside the user's organisation is invited.
    pub is_external: bool,
}

impl SyncedEvent {
    /// The row this event is stored as in `calendar` of `account`. A refresh
    /// keeps the stored ID, `created_at` and record override.
    pub(super) fn to_row(
        &self,
        account: CalendarAccountId,
        calendar: CalendarId,
        synced_at: DateTime<Utc>,
    ) -> CalendarEvent {
        CalendarEvent {
            id: self.id,
            account_id: account,
            calendar_id: Some(calendar),
            provider_event_id: self.provider_event_id.clone(),
            ical_uid: self.ical_uid.clone(),
            series_id: self.series_id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            start_at: self.start_at,
            end_at: self.end_at,
            is_all_day: self.is_all_day,
            conference_url: self.conference_url.clone(),
            platform: self.platform,
            organizer_email: self.organizer_email.clone(),
            attendees: self.attendees.clone(),
            my_rsvp: self.my_rsvp,
            is_external: self.is_external,
            record_override: None,
            created_at: synced_at,
            updated_at: synced_at,
        }
    }
}

/// What storing a batch changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Events stored for the first time.
    pub events_added: usize,
    /// Stored events whose provider data changed.
    pub events_updated: usize,
    /// Stored events that are gone from the provider.
    pub events_removed: usize,
    /// Accounts and calendars added, changed or removed.
    pub calendars_changed: usize,
}

impl SyncReport {
    /// True when the batch changed anything stored.
    pub fn changed(&self) -> bool {
        self.events_added + self.events_updated + self.events_removed + self.calendars_changed > 0
    }

    /// Counts one event upsert: `written` is the stored ID when a row was
    /// written; it equals the candidate only for a new event.
    pub(super) fn count_upsert(
        &mut self,
        candidate: CalendarEventId,
        written: Option<CalendarEventId>,
    ) {
        match written {
            Some(id) if id == candidate => self.events_added += 1,
            Some(_) => self.events_updated += 1,
            None => {}
        }
    }
}
