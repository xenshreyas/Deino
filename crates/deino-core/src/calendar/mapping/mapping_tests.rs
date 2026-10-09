//! How a source's events become stored events.

use deino_domain::{CalendarProvider, ConferencePlatform, FakeClock, Rsvp};

use super::*;
use crate::calendar::EventStatus;
use crate::calendar::test_events::{NOW, clock, me, meeting, person, snapshot};

fn context(clock: &FakeClock) -> BatchContext<'_> {
    BatchContext {
        provider: CalendarProvider::Apple,
        window: SyncWindow::around(deino_testkit::at(NOW)),
        clock,
    }
}

/// The one event of a one-event snapshot, as stored.
fn mapped(event: SourceEvent) -> SyncedEvent {
    let clock = clock();
    let (batch, _) = to_batch(&snapshot(vec![event]), &context(&clock));
    batch.accounts[0].calendars[0].events[0].clone()
}

#[test]
fn a_meeting_keeps_its_link_people_answer_and_audience() {
    let event = mapped(meeting("evt-1", 1));

    assert_eq!(event.provider_event_id, "evt-1");
    assert_eq!(event.platform, Some(ConferencePlatform::Zoom));
    assert_eq!(
        event.conference_url.as_deref(),
        Some("https://us02web.zoom.us/j/881234?pwd=abc")
    );
    assert_eq!(event.organizer_email.as_deref(), Some("noor@deino.dev"));
    assert_eq!(event.my_rsvp, Some(Rsvp::Accepted));
    assert!(
        event.is_external,
        "marcus@brightkite.com is outside deino.dev"
    );
    let emails: Vec<_> = event.attendees.iter().map(|a| a.email.as_deref()).collect();
    assert_eq!(
        emails,
        [
            Some("maya@deino.dev"),
            Some("noor@deino.dev"),
            Some("marcus@brightkite.com")
        ]
    );
    assert!(event.attendees[1].is_organizer);
}

#[test]
fn a_meeting_among_colleagues_is_internal() {
    let mut colleagues = meeting("evt-1", 1);
    colleagues.attendees.pop();

    assert!(!mapped(colleagues).is_external);
}

#[test]
fn occurrences_of_a_recurring_event_share_its_series() {
    let mut first = meeting("weekly/2026-10-09", 1);
    let mut second = meeting("weekly/2026-10-16", 7 * 24 + 1);
    for occurrence in [&mut first, &mut second] {
        occurrence.ical_uid = Some("weekly@deino.dev".into());
        occurrence.series_key = Some("weekly@deino.dev".into());
    }
    let one_off = mapped(meeting("evt-1", 2));

    let (first, second) = (mapped(first), mapped(second));

    assert_eq!(first.series_id, second.series_id);
    assert_eq!(first.series_id.unwrap().as_str(), "weekly@deino.dev");
    assert_eq!(one_off.series_id, None);
    assert_ne!(first.provider_event_id, second.provider_event_id);
}

#[test]
fn rooms_resources_and_google_calendars_are_not_attendees() {
    let mut event = meeting("evt-1", 1);
    let mut room = person("blue-room@deino.dev", Some(Rsvp::Accepted));
    room.kind = ParticipantKind::Room;
    event.attendees.push(room);
    event.attendees.push(person(
        "c_188abc@resource.calendar.google.com",
        Some(Rsvp::Accepted),
    ));

    let emails: Vec<_> = mapped(event)
        .attendees
        .into_iter()
        .filter_map(|attendee| attendee.email)
        .collect();

    assert_eq!(
        emails,
        ["maya@deino.dev", "noor@deino.dev", "marcus@brightkite.com"]
    );
}

#[test]
fn an_organiser_listed_apart_joins_the_attendees_first() {
    let mut event = meeting("evt-1", 1);
    event.attendees.remove(1);

    let attendees = mapped(event).attendees;

    assert_eq!(attendees[0].email.as_deref(), Some("noor@deino.dev"));
    assert!(attendees[0].is_organizer);
    assert_eq!(attendees[0].rsvp, Some(Rsvp::Accepted));
}

#[test]
fn the_users_answer_comes_from_their_entry_or_their_organising() {
    let mut declined = meeting("declined", 1);
    declined.attendees[0] = me(Some(Rsvp::Declined));
    let mut organised = meeting("organised", 1);
    organised.organizer = Some(me(None));
    organised.attendees.remove(0);

    assert_eq!(mapped(declined).my_rsvp, Some(Rsvp::Declined));
    assert_eq!(mapped(organised).my_rsvp, Some(Rsvp::Accepted));
}

#[test]
fn a_solo_event_has_no_attendees_answer_or_audience() {
    let mut solo = meeting("solo", 1);
    (solo.organizer, solo.attendees, solo.notes) = (None, Vec::new(), None);

    let solo = mapped(solo);

    assert!(solo.attendees.is_empty());
    assert_eq!(
        (solo.my_rsvp, solo.is_external, solo.platform),
        (None, false, None)
    );
}

#[test]
fn cancelled_events_and_own_time_blocks_are_dropped_but_calendars_stay() {
    let mut cancelled = meeting("cancelled", 1);
    cancelled.status = EventStatus::Cancelled;
    let mut ooo = meeting("ooo", 2);
    ooo.title = "Out of office".into();
    let clock = clock();

    let (batch, _) = to_batch(&snapshot(vec![cancelled, ooo]), &context(&clock));

    let calendar = &batch.accounts[0].calendars[0];
    assert_eq!(calendar.title, "Work");
    assert!(calendar.events.is_empty());
}

#[test]
fn an_event_that_ends_before_it_starts_is_stored_as_instant() {
    let mut backwards = meeting("evt-1", 1);
    backwards.end = backwards.start - chrono::TimeDelta::minutes(5);

    let event = mapped(backwards);

    assert_eq!(event.end_at, event.start_at);
}
