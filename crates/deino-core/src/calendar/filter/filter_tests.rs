//! The hiding and de-duplication rules.

use std::collections::BTreeSet;

use deino_domain::{CalendarEventId, CalendarId, Rsvp};

use super::*;
use crate::calendar::test_events::{clock, hours, meeting, stored};

fn range() -> TimeRange {
    TimeRange {
        start: hours(0),
        end: hours(24),
    }
}

fn titles(events: &[CalendarEvent]) -> Vec<&str> {
    events.iter().map(|event| event.title.as_str()).collect()
}

#[test]
fn blocks_of_the_users_own_time_are_not_kept() {
    let cases = [
        ("Out of office", Availability::Busy, false),
        ("OOO - dentist", Availability::Busy, false),
        ("❇️ Focus Time (via Clockwise)", Availability::Busy, false),
        ("Focus", Availability::Busy, false),
        ("Working location: Home", Availability::Free, false),
        ("Home", Availability::Free, true),
        ("Vacation", Availability::Unavailable, false),
    ];
    for (title, availability, all_day) in cases {
        let mut event = meeting("block", 1);
        (event.title, event.availability, event.all_day) = (title.into(), availability, all_day);
        assert!(!keeps(&event), "{title:?} should be hidden");
    }
}

#[test]
fn meetings_with_lookalike_titles_are_kept() {
    for title in [
        "Office hours",
        "Home page review",
        "OOOps retro",
        "Focus group with customers",
        "Pricing review",
    ] {
        let mut event = meeting("event", 1);
        event.title = title.into();
        assert!(keeps(&event), "{title:?} should be kept");
    }
}

#[test]
fn the_providers_event_type_beats_the_title() {
    let mut focus = meeting("focus", 1);
    focus.kind = Some(EventKind::Focus);
    let mut titled = meeting("titled", 1);
    titled.title = "Out of office".into();
    titled.kind = Some(EventKind::Default);

    assert!(!keeps(&focus));
    assert!(keeps(&titled));
}

#[test]
fn cancelled_events_are_not_kept() {
    let mut cancelled = meeting("cancelled", 1);
    cancelled.status = EventStatus::Cancelled;
    assert!(!keeps(&cancelled));
}

#[test]
fn declined_events_and_hidden_calendars_are_left_out() {
    let work = CalendarId::generate(&clock());
    let mut declined = stored("Declined", 1);
    declined.my_rsvp = Some(Rsvp::Declined);
    let mut hidden = stored("Hidden", 2);
    hidden.calendar_id = Some(work);
    let shown = stored("Shown", 3);

    let listed = agenda(
        vec![declined, hidden, shown],
        &BTreeSet::from([work]),
        &range(),
    );

    assert_eq!(titles(&listed), ["Shown"]);
}

#[test]
fn ongoing_events_are_listed_and_finished_or_later_ones_are_not() {
    let mut ongoing = stored("Ongoing", -1);
    ongoing.end_at = hours(1);
    let finished = stored("Finished", -2);
    let tomorrow = stored("Tomorrow", 25);

    let listed = agenda(
        vec![tomorrow, finished, ongoing],
        &BTreeSet::new(),
        &range(),
    );

    assert_eq!(titles(&listed), ["Ongoing"]);
}

#[test]
fn the_same_invitation_on_two_calendars_shows_once_with_the_richer_copy() {
    let mut bare = stored("Pricing review", 1);
    bare.ical_uid = Some("pricing@deino.dev".into());
    let mut rich = bare.clone();
    rich.id = CalendarEventId::generate(&clock());
    rich.conference_url = Some("https://zoom.us/j/1".into());
    let mut next_week = bare.clone();
    next_week.start_at = hours(7 * 24);

    let listed = agenda(
        vec![bare, rich.clone(), next_week],
        &BTreeSet::new(),
        &TimeRange {
            start: hours(0),
            end: hours(8 * 24),
        },
    );

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, rich.id);
}

#[test]
fn listed_events_are_ordered_by_start() {
    let listed = agenda(
        vec![stored("Third", 3), stored("First", 1), stored("Second", 2)],
        &BTreeSet::new(),
        &range(),
    );
    assert_eq!(titles(&listed), ["First", "Second", "Third"]);
}
