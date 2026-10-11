//! Calendar accounts, their calendars and the events synced from them.

use chrono::{DateTime, Utc};
use deino_domain::{
    CalendarAccountId, CalendarEventId, CalendarId, CalendarProvider, ConferencePlatform, Rsvp,
    SeriesId,
};
use serde::{Deserialize, Serialize};

use super::CalendarColor;

/// A calendar source the user connected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarAccount {
    /// The account's ID.
    pub id: CalendarAccountId,
    /// Where its events come from.
    pub provider: CalendarProvider,
    /// What Settings shows: the account's email, or "On My Mac" for `EventKit`.
    /// Unique per provider.
    pub name: String,
    /// When the account was connected.
    pub created_at: DateTime<Utc>,
}

/// One calendar inside an account ("Work", "Family", "US Holidays").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    /// The calendar's ID in Deino.
    pub id: CalendarId,
    /// The account it belongs to.
    pub account_id: CalendarAccountId,
    /// The provider's ID for it; unique per account, so syncs upsert.
    pub provider_calendar_id: String,
    /// Its name; may be empty.
    pub title: String,
    /// The colour the provider shows it in.
    pub color: Option<CalendarColor>,
    /// When it was first synced.
    pub created_at: DateTime<Utc>,
    /// When a sync last changed it.
    pub updated_at: DateTime<Utc>,
}

/// One event, as last synced from its calendar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    /// The event's ID in Deino.
    pub id: CalendarEventId,
    /// The account it was synced from.
    pub account_id: CalendarAccountId,
    /// The calendar it belongs to; `None` only for events stored before
    /// ORG-01 knew calendars.
    pub calendar_id: Option<CalendarId>,
    /// The provider's ID for it; unique per account, so syncs upsert. A
    /// recurring event's occurrences each have their own.
    pub provider_event_id: String,
    /// The iCalendar UID shared by every copy of the invitation.
    pub ical_uid: Option<String>,
    /// The recurring series it belongs to.
    pub series_id: Option<SeriesId>,
    /// The event's title; may be empty.
    pub title: String,
    /// The event's description; may be empty.
    pub description: String,
    /// When it starts.
    pub start_at: DateTime<Utc>,
    /// When it ends; never before `start_at`.
    pub end_at: DateTime<Utc>,
    /// True for an event that spans whole days; `start_at` and `end_at` are
    /// then the user's local midnights.
    pub is_all_day: bool,
    /// The video-call link, if any.
    pub conference_url: Option<String>,
    /// The service behind `conference_url`.
    pub platform: Option<ConferencePlatform>,
    /// Who organised it.
    pub organizer_email: Option<String>,
    /// The invitation list as the provider sent it.
    pub attendees: Vec<EventAttendee>,
    /// The user's own answer.
    pub my_rsvp: Option<Rsvp>,
    /// True when someone outside the user's domains is invited.
    pub is_external: bool,
    /// `None` follows the auto-record rules; `Some(true)` always records,
    /// `Some(false)` never does. Set by the user, so syncs keep it.
    pub record_override: Option<bool>,
    /// When the event was first synced.
    pub created_at: DateTime<Utc>,
    /// When a sync last changed it.
    pub updated_at: DateTime<Utc>,
}

/// One invitee of a calendar event. Stored as JSON in
/// `calendar_events.attendees_json`; linking the event to a meeting turns
/// these into people and `meeting_attendees` rows (ORG-03, ORG-04).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EventAttendee {
    /// Their email address, as the provider wrote it.
    pub email: Option<String>,
    /// Their display name.
    pub name: Option<String>,
    /// Their answer to the invitation.
    pub rsvp: Option<Rsvp>,
    /// True for the organiser.
    pub is_organizer: bool,
}
