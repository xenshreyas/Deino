//! Back-to-back meetings (CAP-07, Granola §2.5): when the next calendar
//! event begins while a recording runs and other people are invited to it,
//! the recording most likely ran from one meeting into the next (Zoom kept
//! the microphone between calls), so the user is asked "Start new note for
//! …?". Pure, over the events the calendar adapter hands in.

use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{CalendarEventId, SeriesId};

use crate::calendar::is_linkable;

/// How long after the next event's start it is still offered.
pub const OFFER_WINDOW: TimeDelta = TimeDelta::minutes(5);

/// A calendar event, as auto-stop sees it. The title is meeting content:
/// shown, never logged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventWindow {
    /// The event.
    pub id: CalendarEventId,
    /// Its title.
    pub title: String,
    /// When it starts.
    pub starts_at: DateTime<Utc>,
    /// When it ends.
    pub ends_at: DateTime<Utc>,
    /// Its recurring series, if any.
    pub series_id: Option<SeriesId>,
    /// The invitees' email addresses, lowercased, without the user's own.
    pub attendees: Vec<String>,
}

impl EventWindow {
    /// The invitees as a set.
    fn invitees(&self) -> BTreeSet<&str> {
        self.attendees.iter().map(String::as_str).collect()
    }
}

/// The event a recording that started at `started_at` belongs to: the one
/// it names, else the one a call at that time links to (CAP-06's window).
pub fn current_event(
    events: &[EventWindow],
    named: Option<CalendarEventId>,
    started_at: DateTime<Utc>,
) -> Option<EventWindow> {
    if let Some(named) = named {
        return events.iter().find(|event| event.id == named).cloned();
    }
    events
        .iter()
        .filter(|event| is_linkable(event.starts_at, started_at))
        .min_by_key(|event| (event.starts_at - started_at).abs())
        .cloned()
}

/// The meeting to offer a new note for at `now`, if any: an event that
/// began within the last [`OFFER_WINDOW`], after the recording's own event
/// (not one the recording started for), with invitees, and a different
/// set of them than the current event's.
pub fn next_meeting<'a>(
    now: DateTime<Utc>,
    current: Option<&EventWindow>,
    started_at: DateTime<Utc>,
    events: &'a [EventWindow],
) -> Option<&'a EventWindow> {
    events
        .iter()
        .filter(|event| current.is_none_or(|current| current.id != event.id))
        .filter(|event| event.starts_at <= now && now - event.starts_at < OFFER_WINDOW)
        .filter(|event| !is_linkable(event.starts_at, started_at))
        .filter(|event| event.starts_at > started_at)
        .filter(|event| !event.attendees.is_empty())
        .filter(|event| current.is_none_or(|current| current.invitees() != event.invitees()))
        .max_by_key(|event| event.starts_at)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(minutes: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
    }

    fn event(last: u8, start: i64, attendees: &[&str]) -> EventWindow {
        EventWindow {
            id: format!("01926b2c-0000-7000-8000-0000000000{last:02}")
                .parse()
                .unwrap(),
            title: format!("Event {last}"),
            starts_at: at(start),
            ends_at: at(start + 30),
            series_id: None,
            attendees: attendees.iter().map(|a| (*a).to_owned()).collect(),
        }
    }

    #[test]
    fn the_next_meeting_with_other_people_is_offered_once_it_begins() {
        let design = event(1, 0, &["ana@example.com"]);
        let sales = event(2, 30, &["bo@example.com"]);
        let events = [design.clone(), sales.clone()];

        assert_eq!(next_meeting(at(29), Some(&design), at(0), &events), None);
        assert_eq!(
            next_meeting(at(31), Some(&design), at(0), &events),
            Some(&sales)
        );
        assert_eq!(next_meeting(at(36), Some(&design), at(0), &events), None);
    }

    #[test]
    fn the_same_people_meeting_again_is_one_conversation() {
        let first = event(1, 0, &["ana@example.com", "bo@example.com"]);
        let second = event(2, 30, &["bo@example.com", "ana@example.com"]);
        let events = [first.clone(), second];

        assert_eq!(next_meeting(at(31), Some(&first), at(0), &events), None);
    }

    #[test]
    fn a_solo_block_is_never_offered() {
        let first = event(1, 0, &["ana@example.com"]);
        let focus = event(2, 30, &[]);
        let events = [first.clone(), focus];

        assert_eq!(next_meeting(at(31), Some(&first), at(0), &events), None);
    }

    #[test]
    fn an_ad_hoc_recording_is_not_split_by_the_event_it_started_for() {
        let standup = event(1, 3, &["ana@example.com"]);
        let events = [standup];

        assert_eq!(next_meeting(at(4), None, at(0), &events), None);
        assert_eq!(
            current_event(&events, None, at(0)).unwrap().id,
            events[0].id
        );
    }

    #[test]
    fn an_ad_hoc_recording_running_into_a_meeting_is_offered_it() {
        let review = event(1, 20, &["ana@example.com"]);
        let events = [review.clone()];

        assert_eq!(current_event(&events, None, at(0)), None);
        assert_eq!(next_meeting(at(21), None, at(0), &events), Some(&review));
    }

    #[test]
    fn the_named_event_is_the_current_one() {
        let first = event(1, -40, &["ana@example.com"]);
        let second = event(2, 0, &["bo@example.com"]);
        let events = [first.clone(), second];

        assert_eq!(current_event(&events, Some(first.id), at(0)), Some(first));
    }
}
