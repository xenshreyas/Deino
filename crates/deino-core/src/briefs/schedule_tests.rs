use chrono::TimeDelta;
use deino_domain::{FixedZone, Rsvp};
use deino_testkit::at;

use super::*;

fn pacific() -> FixedZone {
    FixedZone::hours(-7).unwrap()
}

/// An event at 10:00 Pacific on Monday 12 October 2026.
const TEN_AM: &str = "2026-10-12T17:00:00Z";

#[test]
fn nothing_is_due_before_six_unless_the_event_is_within_the_hour() {
    let zone = pacific();
    // 05:59 Pacific.
    assert_eq!(
        due_trigger(at(TEN_AM), at("2026-10-12T12:59:00Z"), &zone),
        None
    );
    // An event at 06:30 is due at 05:30, an hour before.
    let early = at("2026-10-12T13:30:00Z");
    assert_eq!(
        due_trigger(early, at("2026-10-12T12:30:00Z"), &zone),
        Some(BriefTrigger::BeforeEvent)
    );
}

#[test]
fn the_morning_run_covers_the_rest_of_the_day_from_six() {
    let zone = pacific();
    for now in [
        "2026-10-12T13:00:00Z",
        "2026-10-12T15:00:00Z",
        "2026-10-12T15:59:00Z",
    ] {
        assert_eq!(
            due_trigger(at(TEN_AM), at(now), &zone),
            Some(BriefTrigger::Morning),
            "{now}"
        );
    }
    // Tomorrow's 10:00 is not today's business.
    let tomorrow = at(TEN_AM) + TimeDelta::days(1);
    assert_eq!(
        due_trigger(tomorrow, at("2026-10-12T15:00:00Z"), &zone),
        None
    );
}

#[test]
fn the_hour_before_wins_and_nothing_runs_once_the_meeting_started() {
    let zone = pacific();
    assert_eq!(
        due_trigger(at(TEN_AM), at("2026-10-12T16:00:00Z"), &zone),
        Some(BriefTrigger::BeforeEvent)
    );
    assert_eq!(
        due_trigger(at(TEN_AM), at("2026-10-12T16:59:00Z"), &zone),
        Some(BriefTrigger::BeforeEvent)
    );
    assert_eq!(due_trigger(at(TEN_AM), at(TEN_AM), &zone), None);
}

#[test]
fn the_window_reaches_local_midnight_or_the_next_hour() {
    let zone = pacific();
    let morning = at("2026-10-12T13:30:00Z");
    assert_eq!(
        due_window(morning, &zone),
        (morning, at("2026-10-13T07:00:00Z"))
    );
    // 23:30 Pacific: the next hour reaches past midnight.
    let late = at("2026-10-13T06:30:00Z");
    assert_eq!(due_window(late, &zone), (late, late + LEAD_TIME));
}

fn event(attendees: usize, external: bool) -> CalendarEvent {
    let attendee = crate::calendar::EventAttendee {
        email: Some("a@b.example".into()),
        name: None,
        rsvp: None,
        is_organizer: false,
    };
    CalendarEvent {
        id: deino_domain::CalendarEventId::generate(&deino_domain::FakeClock::new(at(TEN_AM))),
        account_id: deino_domain::CalendarAccountId::generate(&deino_domain::FakeClock::new(at(
            TEN_AM,
        ))),
        calendar_id: None,
        provider_event_id: "x".into(),
        ical_uid: None,
        series_id: None,
        title: "Sync".into(),
        description: String::new(),
        start_at: at(TEN_AM),
        end_at: at(TEN_AM),
        is_all_day: false,
        conference_url: None,
        platform: None,
        organizer_email: None,
        attendees: vec![attendee; attendees],
        my_rsvp: None,
        is_external: external,
        record_override: None,
        created_at: at(TEN_AM),
        updated_at: at(TEN_AM),
    }
}

#[test]
fn external_meetings_qualify_and_groups_widen_it_to_two_attendees() {
    assert!(qualifies(&event(2, true), BriefAudience::External));
    assert!(!qualifies(&event(3, false), BriefAudience::External));
    assert!(qualifies(&event(2, false), BriefAudience::Groups));
    assert!(!qualifies(&event(1, false), BriefAudience::Groups));
}

#[test]
fn declined_and_all_day_events_never_qualify() {
    let mut declined = event(2, true);
    declined.my_rsvp = Some(Rsvp::Declined);
    let mut all_day = event(2, true);
    all_day.is_all_day = true;

    assert!(!qualifies(&declined, BriefAudience::External));
    assert!(!qualifies(&all_day, BriefAudience::Groups));
}
