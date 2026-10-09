//! Builders shared by the calendar tests: a source's events as `EventKit`
//! would report them, and stored events.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{CalendarAccountId, CalendarEventId, FakeClock, Rsvp};
use deino_testkit::at;

use super::{
    Availability, CalendarColor, CalendarEvent, EventAttendee, EventStatus, Participant,
    ParticipantKind, SourceAccount, SourceCalendar, SourceEvent, SourceSnapshot,
};

/// The tests' "now".
pub(super) const NOW: &str = "2026-10-09T09:00:00Z";
/// The user's work address in the fixtures.
pub(super) const ME: &str = "maya@deino.dev";

/// A clock frozen at [`NOW`].
pub(super) fn clock() -> FakeClock {
    FakeClock::new(at(NOW))
}

/// [`NOW`] plus `hours`.
pub(super) fn hours(hours: i64) -> DateTime<Utc> {
    at(NOW) + TimeDelta::hours(hours)
}

/// A person invited with `rsvp`.
pub(super) fn person(email: &str, rsvp: Option<Rsvp>) -> Participant {
    Participant {
        email: Some(format!("mailto:{email}")),
        name: None,
        rsvp,
        kind: ParticipantKind::Person,
        is_self: false,
    }
}

/// The user's own entry, answered with `rsvp`.
pub(super) fn me(rsvp: Option<Rsvp>) -> Participant {
    Participant {
        is_self: true,
        ..person(ME, rsvp)
    }
}

/// A 30-minute meeting starting `starts_in` hours from [`NOW`]: organised by
/// a colleague, with the user and an outside guest invited, on Zoom.
pub(super) fn meeting(provider_id: &str, starts_in: i64) -> SourceEvent {
    SourceEvent {
        provider_event_id: provider_id.into(),
        ical_uid: Some(format!("{provider_id}@deino.dev")),
        series_key: None,
        title: "Pricing review".into(),
        notes: Some("Join Zoom Meeting\nhttps://us02web.zoom.us/j/881234?pwd=abc".into()),
        location: None,
        url: None,
        conference_url: None,
        start: hours(starts_in),
        end: hours(starts_in) + TimeDelta::minutes(30),
        all_day: false,
        status: EventStatus::Confirmed,
        kind: None,
        availability: Availability::Busy,
        organizer: Some(person("noor@deino.dev", None)),
        attendees: vec![
            me(Some(Rsvp::Accepted)),
            person("noor@deino.dev", Some(Rsvp::Accepted)),
            person("marcus@brightkite.com", Some(Rsvp::Tentative)),
        ],
    }
}

/// One iCloud account with a "Work" calendar holding `events`.
pub(super) fn snapshot(events: Vec<SourceEvent>) -> SourceSnapshot {
    SourceSnapshot {
        accounts: vec![SourceAccount {
            name: "iCloud".into(),
            calendars: vec![SourceCalendar {
                provider_id: "work".into(),
                title: "Work".into(),
                color: CalendarColor::parse("#34AADC").ok(),
                events,
            }],
        }],
    }
}

/// A stored event `title` starting `starts_in` hours from [`NOW`].
pub(super) fn stored(title: &str, starts_in: i64) -> CalendarEvent {
    let clock = clock();
    CalendarEvent {
        id: CalendarEventId::generate(&clock),
        account_id: CalendarAccountId::generate(&clock),
        calendar_id: None,
        provider_event_id: title.into(),
        ical_uid: None,
        series_id: None,
        title: title.into(),
        description: String::new(),
        start_at: hours(starts_in),
        end_at: hours(starts_in) + TimeDelta::minutes(30),
        is_all_day: false,
        conference_url: None,
        platform: None,
        organizer_email: None,
        attendees: vec![EventAttendee {
            email: Some(ME.into()),
            name: None,
            rsvp: Some(Rsvp::Accepted),
            is_organizer: true,
        }],
        my_rsvp: Some(Rsvp::Accepted),
        is_external: false,
        record_override: None,
        created_at: at(NOW),
        updated_at: at(NOW),
    }
}
