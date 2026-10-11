//! Calendar association: a detected call that starts near a calendar event
//! is assumed to be that event (Granola §2.3), so the prompt names it.
//!
//! The link window is an event that started less than 15 minutes ago or
//! starts within 5 minutes. The app wires the synced calendar (ORG-01,
//! `features::calendar::SyncedCalendar`); [`NoCalendar`] is for tests and
//! builds without calendars.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::CalendarEventId;
use serde::Serialize;

/// How long after an event's start a call still belongs to it.
pub const LINK_AFTER_START: TimeDelta = TimeDelta::minutes(15);

/// How long before an event's start a call already belongs to it.
pub const LINK_BEFORE_START: TimeDelta = TimeDelta::minutes(5);

/// A calendar event a detection is linked to. The title is meeting content:
/// shown in the prompt, never logged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEvent {
    /// The event.
    pub id: CalendarEventId,
    /// Its title.
    pub title: String,
    /// When it starts.
    pub starts_at: DateTime<Utc>,
}

/// Finds the calendar event a call starting `now` belongs to.
pub trait CalendarLookup: Send + Sync {
    /// The event that [`is_linkable`] at `now`, the nearest start first.
    fn linked_event(&self, now: DateTime<Utc>) -> Option<LinkedEvent>;
}

/// The lookup before calendars exist: never links an event.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoCalendar;

impl CalendarLookup for NoCalendar {
    fn linked_event(&self, _now: DateTime<Utc>) -> Option<LinkedEvent> {
        None
    }
}

/// Whether a call at `now` belongs to an event starting at `starts_at`.
pub fn is_linkable(starts_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    let since_start = now - starts_at;
    since_start < LINK_AFTER_START && -since_start <= LINK_BEFORE_START
}

/// The event among `events` that a call at `now` belongs to: the one whose
/// start is closest to `now`. For adapters over a list of events.
pub fn pick_linked(
    events: impl IntoIterator<Item = LinkedEvent>,
    now: DateTime<Utc>,
) -> Option<LinkedEvent> {
    events
        .into_iter()
        .filter(|event| is_linkable(event.starts_at, now))
        .min_by_key(|event| (now - event.starts_at).abs())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 9, 10, minute, 0).unwrap()
    }

    fn id(last: u8) -> CalendarEventId {
        format!("01926b2c-0000-7000-8000-0000000000{last:02}")
            .parse()
            .unwrap()
    }

    fn event(last: u8, starts_at: DateTime<Utc>) -> LinkedEvent {
        LinkedEvent {
            id: id(last),
            title: "Weekly sync".to_owned(),
            starts_at,
        }
    }

    #[test]
    fn a_call_links_up_to_fifteen_minutes_after_and_five_before_the_start() {
        let start = at(30);
        assert!(is_linkable(start, at(30)));
        assert!(is_linkable(start, at(44)));
        assert!(!is_linkable(start, at(45)));
        assert!(is_linkable(start, at(25)));
        assert!(!is_linkable(start, at(24)));
    }

    #[test]
    fn the_event_starting_closest_to_now_wins() {
        let picked = pick_linked(
            [event(1, at(20)), event(2, at(33)), event(3, at(50))],
            at(30),
        );

        assert_eq!(picked.unwrap().id, id(2));
    }

    #[test]
    fn without_calendars_nothing_links() {
        assert_eq!(NoCalendar.linked_event(at(0)), None);
    }
}
