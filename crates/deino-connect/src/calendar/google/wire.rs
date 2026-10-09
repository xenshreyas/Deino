//! The Google Calendar API v3 JSON this connector reads: only the fields it
//! uses, every one optional (Google omits empty fields).

use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::Deserialize;

/// A page of `users/me/calendarList`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CalendarListPage {
    #[serde(default)]
    pub(super) items: Vec<CalendarEntry>,
    pub(super) next_page_token: Option<String>,
}

/// One calendar the account sees.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CalendarEntry {
    pub(super) id: String,
    pub(super) summary: Option<String>,
    pub(super) summary_override: Option<String>,
    pub(super) background_color: Option<String>,
    #[serde(default)]
    pub(super) primary: bool,
    #[serde(default)]
    pub(super) deleted: bool,
}

/// A page of `calendars/{id}/events`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EventsPage {
    #[serde(default)]
    pub(super) items: Vec<Event>,
    pub(super) next_page_token: Option<String>,
}

/// One event, or one instance of a recurring event (`singleEvents=true`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Event {
    pub(super) id: String,
    pub(super) status: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) description: Option<String>,
    pub(super) location: Option<String>,
    pub(super) hangout_link: Option<String>,
    pub(super) conference_data: Option<ConferenceData>,
    pub(super) start: Option<When>,
    pub(super) end: Option<When>,
    #[serde(rename = "iCalUID")]
    pub(super) ical_uid: Option<String>,
    pub(super) recurring_event_id: Option<String>,
    #[serde(rename = "eventType")]
    pub(super) kind: Option<String>,
    pub(super) transparency: Option<String>,
    #[serde(default)]
    pub(super) attendees: Vec<Attendee>,
    pub(super) organizer: Option<Person>,
}

/// A start or end: an instant, or a date for all-day events.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct When {
    pub(super) date_time: Option<DateTime<FixedOffset>>,
    pub(super) date: Option<NaiveDate>,
}

/// Structured conference data (Meet, or an add-on's Zoom or Teams).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ConferenceData {
    #[serde(default)]
    pub(super) entry_points: Vec<EntryPoint>,
}

/// One way to join: `video`, `phone`, `sip` or `more`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EntryPoint {
    pub(super) entry_point_type: Option<String>,
    pub(super) uri: Option<String>,
}

/// An invitee.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Attendee {
    pub(super) email: Option<String>,
    pub(super) display_name: Option<String>,
    pub(super) response_status: Option<String>,
    #[serde(rename = "self", default)]
    pub(super) is_self: bool,
    #[serde(default)]
    pub(super) resource: bool,
}

/// The organiser.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Person {
    pub(super) email: Option<String>,
    pub(super) display_name: Option<String>,
    #[serde(rename = "self", default)]
    pub(super) is_self: bool,
}
