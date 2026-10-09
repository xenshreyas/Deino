//! What Deino hides from a calendar, and how duplicates collapse.
//!
//! Two moments apply the rules (Granola's, `docs/research/granola.md` §6.5):
//!
//! - **At sync** ([`keeps`]): cancelled events and blocks of the user's own
//!   time (out of office, focus time, working location) are never stored.
//! - **On read** ([`agenda`]): declined events and hidden calendars are left
//!   out, and the same invitation seen through two calendars (a shared
//!   calendar added twice, or one account reached through macOS Calendar and
//!   directly) shows once. Both stay stored, so an RSVP change or showing a
//!   calendar again needs no sync.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use deino_domain::{CalendarId, Rsvp};
use serde::Deserialize;

use super::CalendarEvent;
use super::source::{Availability, EventKind, EventStatus, SourceEvent};

/// Titles of all-day "where I work today" entries.
const WORKING_LOCATION_TITLES: &[&str] = &[
    "home",
    "office",
    "remote",
    "wfh",
    "working from home",
    "working remotely",
];

/// A span of time to list events in: `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    /// The first instant listed.
    pub start: DateTime<Utc>,
    /// The instant listing stops (exclusive).
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// True when `[start, end)` overlaps this range.
    fn overlaps(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        start < self.end && (end > self.start || (start == end && start >= self.start))
    }
}

/// True when a synced event is worth storing: it takes place and it is a
/// meeting or appointment rather than a block of the user's own time.
pub fn keeps(event: &SourceEvent) -> bool {
    event.status != EventStatus::Cancelled && kind_of(event) == EventKind::Default
}

/// The event's kind: the provider's word for it, or else what its title and
/// availability say.
pub fn kind_of(event: &SourceEvent) -> EventKind {
    event
        .kind
        .unwrap_or_else(|| infer_kind(&event.title, event.availability, event.all_day))
}

/// What a block is, judged by its title and availability, for providers
/// with no event types (`EventKit`).
fn infer_kind(title: &str, availability: Availability, all_day: bool) -> EventKind {
    let title = normalised(title);
    let starts = |prefix: &str| title.starts_with(prefix);
    let is_ooo = title == "ooo" || starts("ooo ") || starts("ooo:") || starts("ooo-");
    if is_ooo || starts("out of office") || starts("out of the office") {
        return EventKind::OutOfOffice;
    }
    if availability == Availability::Unavailable {
        return EventKind::OutOfOffice;
    }
    if title == "focus" || starts("focus time") || starts("focus block") {
        return EventKind::Focus;
    }
    let free_all_day = all_day && availability == Availability::Free;
    if starts("working location") || (free_all_day && WORKING_LOCATION_TITLES.contains(&&*title)) {
        return EventKind::WorkingLocation;
    }
    EventKind::Default
}

/// `title` in lower case without leading symbols (`❇️ Focus Time`) or
/// surrounding whitespace.
fn normalised(title: &str) -> String {
    title
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim()
        .to_lowercase()
}

/// The events to show in `range`: overlapping it, not declined, not on a
/// hidden calendar, each invitation once, by start time.
pub fn agenda(
    events: Vec<CalendarEvent>,
    hidden: &BTreeSet<CalendarId>,
    range: &TimeRange,
) -> Vec<CalendarEvent> {
    let shown = events.into_iter().filter(|event| {
        range.overlaps(event.start_at, event.end_at)
            && event.my_rsvp != Some(Rsvp::Declined)
            && event.calendar_id.is_none_or(|id| !hidden.contains(&id))
    });
    let mut listed = without_duplicates(shown);
    listed.sort_by_key(|event| (event.start_at, event.id));
    listed
}

/// One event per invitation: copies share their iCalendar UID (or, without
/// one, their title) and start time. The richest copy stays.
fn without_duplicates(events: impl Iterator<Item = CalendarEvent>) -> Vec<CalendarEvent> {
    let mut kept: BTreeMap<(String, DateTime<Utc>), CalendarEvent> = BTreeMap::new();
    for event in events {
        let identity = event
            .ical_uid
            .clone()
            .unwrap_or_else(|| format!("title:{}", event.title.to_lowercase()));
        let key = (identity, event.start_at);
        match kept.get(&key) {
            Some(existing) if richness(existing) >= richness(&event) => {}
            _ => {
                kept.insert(key, event);
            }
        }
    }
    kept.into_values().collect()
}

/// How much a copy of an event knows: a join link first, then attendees.
fn richness(event: &CalendarEvent) -> (bool, usize) {
    (event.conference_url.is_some(), event.attendees.len())
}

#[cfg(test)]
mod filter_tests;
