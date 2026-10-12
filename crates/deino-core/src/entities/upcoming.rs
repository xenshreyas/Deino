//! Calendar events coming up with a person or a company.
//!
//! An event's invitees are an opaque list stored with it (ADR 0012), so the
//! next [`UPCOMING_DAYS`] days of events are read and filtered here; that
//! window holds a few dozen events, not the whole calendar.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{Email, Rsvp};

use super::UpcomingEvent;
use crate::calendar::CalendarEvent;

/// How far ahead entity pages look for events.
pub const UPCOMING_DAYS: i64 = 14;

/// Events listed on a page, at most.
pub const MAX_UPCOMING: usize = 5;

/// The window entity pages read: from `now` for [`UPCOMING_DAYS`].
pub(super) fn window(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    (now, now + TimeDelta::days(UPCOMING_DAYS))
}

/// The events in `events` that invite someone `wanted` accepts, in start
/// order, at most [`MAX_UPCOMING`]. Events the user declined are left out.
pub(super) fn upcoming_with(
    events: &[CalendarEvent],
    wanted: impl Fn(&Email) -> bool,
) -> Vec<UpcomingEvent> {
    let invites = |event: &&CalendarEvent| {
        event.my_rsvp != Some(Rsvp::Declined)
            && event
                .attendees
                .iter()
                .filter_map(|attendee| Email::parse(attendee.email.as_deref()?).ok())
                .any(|email| wanted(&email))
    };
    events
        .iter()
        .filter(invites)
        .take(MAX_UPCOMING)
        .map(|event| UpcomingEvent {
            id: event.id,
            title: event.title.clone(),
            start_at: event.start_at,
            end_at: event.end_at,
            conference_url: event.conference_url.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use deino_domain::{CalendarAccountId, CalendarEventId, Clock, FakeClock};
    use deino_testkit::at;

    use super::*;
    use crate::calendar::EventAttendee;

    fn event(title: &str, emails: &[&str], my_rsvp: Option<Rsvp>) -> CalendarEvent {
        let clock = FakeClock::new(at("2026-10-12T09:00:00Z"));
        CalendarEvent {
            id: CalendarEventId::generate(&clock),
            account_id: CalendarAccountId::generate(&clock),
            calendar_id: None,
            provider_event_id: title.into(),
            ical_uid: None,
            series_id: None,
            title: title.into(),
            description: String::new(),
            start_at: clock.now(),
            end_at: clock.now(),
            is_all_day: false,
            conference_url: None,
            platform: None,
            organizer_email: None,
            attendees: emails
                .iter()
                .map(|email| EventAttendee {
                    email: Some((*email).to_owned()),
                    name: None,
                    rsvp: None,
                    is_organizer: false,
                })
                .collect(),
            my_rsvp,
            is_external: false,
            record_override: None,
            created_at: clock.now(),
            updated_at: clock.now(),
        }
    }

    #[test]
    fn events_inviting_the_person_are_kept_unless_the_user_declined() {
        let events = [
            event("Sync", &["Maya@Deino.dev"], None),
            event("Offsite", &["maya@deino.dev"], Some(Rsvp::Declined)),
            event("Other", &["tom@deino.dev"], None),
        ];
        let maya = Email::parse("maya@deino.dev").unwrap();

        let upcoming = upcoming_with(&events, |email| *email == maya);

        let titles: Vec<&str> = upcoming.iter().map(|event| event.title.as_str()).collect();
        assert_eq!(titles, ["Sync"]);
    }

    #[test]
    fn a_page_lists_a_handful_of_events() {
        let events: Vec<CalendarEvent> = (0..10)
            .map(|n| event(&format!("Event {n}"), &["owen@fernhollow.co"], None))
            .collect();

        let upcoming = upcoming_with(&events, |email| email.domain() == "fernhollow.co");

        assert_eq!(upcoming.len(), MAX_UPCOMING);
    }
}
