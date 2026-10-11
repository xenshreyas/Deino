//! The nearest-event and suggestion rules, as tables: back to back, all day,
//! declined, under way, about to start and too far away.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::Rsvp;
use deino_testkit::at;

use super::*;
use crate::calendar::CalendarEvent;
use crate::calendar::test_events::stored;

/// 10:00 on the tests' day.
fn ten() -> DateTime<Utc> {
    at("2026-10-09T10:00:00Z")
}

/// `HH:MM` on the tests' day.
fn clock(time: &str) -> DateTime<Utc> {
    at(&format!("2026-10-09T{time}:00Z"))
}

/// What an event in a table is.
#[derive(Clone, Copy)]
enum Kind {
    Timed,
    AllDay,
    Declined,
}

/// An event `title` from `start` lasting `minutes`.
fn event(title: &str, start: &str, minutes: i64, kind: Kind) -> CalendarEvent {
    let start_at = clock(start);
    let mut event = CalendarEvent {
        start_at,
        end_at: start_at + TimeDelta::minutes(minutes),
        ..stored(title, 0)
    };
    match kind {
        Kind::Timed => {}
        Kind::AllDay => {
            event.is_all_day = true;
            event.start_at = at("2026-10-09T00:00:00Z");
            event.end_at = at("2026-10-10T00:00:00Z");
        }
        Kind::Declined => event.my_rsvp = Some(Rsvp::Declined),
    }
    event
}

/// One row: the calendar, when recording starts, and the title linked.
struct Case {
    name: &'static str,
    events: Vec<CalendarEvent>,
    now: &'static str,
    linked: Option<&'static str>,
}

#[rustfmt::skip]
fn cases() -> Vec<Case> {
    use Kind::{AllDay, Declined, Timed};
    let back_to_back = || {
        vec![
            event("Design review", "10:00", 30, Timed),
            event("1:1", "10:30", 30, Timed),
        ]
    };
    vec![
        Case { name: "under way", events: vec![event("Sync", "09:50", 30, Timed)], now: "10:00", linked: Some("Sync") },
        Case { name: "starts in 15 minutes", events: vec![event("Sync", "10:15", 30, Timed)], now: "10:00", linked: Some("Sync") },
        Case { name: "starts in 16 minutes", events: vec![event("Sync", "10:16", 30, Timed)], now: "10:00", linked: None },
        Case { name: "ended a minute ago", events: vec![event("Sync", "09:29", 30, Timed)], now: "10:00", linked: None },
        Case { name: "ends exactly now", events: vec![event("Sync", "09:30", 30, Timed)], now: "10:00", linked: None },
        Case { name: "back to back, early in the first", events: back_to_back(), now: "10:05", linked: Some("Design review") },
        Case { name: "back to back, near the second", events: back_to_back(), now: "10:28", linked: Some("1:1") },
        Case { name: "back to back, at the switch", events: back_to_back(), now: "10:30", linked: Some("1:1") },
        Case { name: "back to back, tie goes to the one under way", events: back_to_back(), now: "10:15", linked: Some("Design review") },
        Case { name: "all-day events never link", events: vec![event("Offsite", "00:00", 0, AllDay)], now: "10:00", linked: None },
        Case { name: "all day beside a meeting", events: vec![event("Offsite", "00:00", 0, AllDay), event("Sync", "10:05", 30, Timed)], now: "10:00", linked: Some("Sync") },
        Case { name: "declined events never link", events: vec![event("Sync", "10:00", 30, Declined)], now: "10:00", linked: None },
        Case { name: "declined beside an accepted one", events: vec![event("Clash", "10:00", 30, Declined), event("Sync", "10:10", 30, Timed)], now: "10:00", linked: Some("Sync") },
        Case { name: "a long meeting under way loses to one starting sooner", events: vec![event("Workshop", "09:00", 180, Timed), event("Sync", "10:10", 30, Timed)], now: "10:00", linked: Some("Sync") },
        Case { name: "no events", events: Vec::new(), now: "10:00", linked: None },
    ]
}

#[test]
fn the_nearest_event_follows_the_table() {
    for case in cases() {
        let linked = nearest_event(&case.events, clock(case.now)).map(|event| event.title.as_str());
        assert_eq!(linked, case.linked, "{}", case.name);
    }
}

#[test]
fn the_link_range_covers_every_event_the_rule_can_pick() {
    let range = link_range(ten());

    assert_eq!(range.start, ten());
    assert!(
        range.end > ten() + LINK_WINDOW,
        "an event starting at the edge overlaps it"
    );
    assert!(range.end <= ten() + LINK_WINDOW + TimeDelta::minutes(1));
}

#[test]
fn suggestions_cover_the_day_before_and_the_hour_after() {
    let range = suggestion_range(ten());

    assert_eq!(range.start, ten() - TimeDelta::hours(24));
    assert_eq!(range.end, ten() + TimeDelta::hours(1));
}

#[test]
fn suggestions_list_the_closest_start_first_without_all_day_or_declined_events() {
    let events = vec![
        event("Yesterday", "00:30", 30, Kind::Timed),
        event("Offsite", "00:00", 0, Kind::AllDay),
        event("Earlier", "09:00", 30, Kind::Timed),
        event("Declined", "10:00", 30, Kind::Declined),
        event("Next", "10:20", 30, Kind::Timed),
        event("Before", "09:40", 30, Kind::Timed),
    ];

    let ranked = rank_suggestions(events, ten());

    let titles: Vec<_> = ranked.iter().map(|event| event.title.as_str()).collect();
    assert_eq!(titles, ["Next", "Before", "Earlier", "Yesterday"]);
}

#[test]
fn suggestions_are_capped() {
    let events = (0..30).map(|n| CalendarEvent {
        start_at: ten() - TimeDelta::minutes(n * 10),
        end_at: ten() - TimeDelta::minutes(n * 10 - 5),
        ..stored(&format!("Event {n}"), 0)
    });

    assert_eq!(
        rank_suggestions(events.collect(), ten()).len(),
        MAX_SUGGESTIONS
    );
}
