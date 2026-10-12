//! Which meetings remind, and that each reminds once.

use chrono::TimeDelta;
use deino_domain::{CalendarId, FakeClock, Rsvp};

use super::{REMINDER_LEAD, ReminderClock, wants_reminder};
use crate::calendar::CalendarEvent;
use crate::calendar::test_events::{NOW, hours, stored};
use crate::settings::ReminderSettings;

/// A two-person meeting starting `seconds` after [`NOW`].
fn meeting(title: &str, seconds: i64) -> CalendarEvent {
    let mut event = stored(title, 0);
    event.start_at = hours(0) + TimeDelta::seconds(seconds);
    event.end_at = event.start_at + TimeDelta::minutes(30);
    let mut guest = event.attendees[0].clone();
    guest.email = Some("noor@deino.dev".into());
    guest.is_organizer = false;
    event.attendees.push(guest);
    event
}

fn titles(due: &[&CalendarEvent]) -> Vec<String> {
    due.iter().map(|event| event.title.clone()).collect()
}

#[test]
fn only_meetings_starting_within_the_lead_remind_and_only_once() {
    let events = [meeting("Soon", 50), meeting("Later", 120), meeting("Started", -10)];
    let settings = ReminderSettings::default();
    let mut clock = ReminderClock::default();

    assert_eq!(titles(&clock.due(&events, hours(0), &settings)), ["Soon"]);
    assert!(clock.due(&events, hours(0) + TimeDelta::seconds(15), &settings).is_empty());

    let later = hours(0) + TimeDelta::seconds(120) - REMINDER_LEAD;
    assert_eq!(titles(&clock.due(&events, later, &settings)), ["Later"]);
}

#[test]
fn a_late_tick_after_sleep_still_reminds_before_the_start() {
    let events = [meeting("Standup", 30)];
    let mut clock = ReminderClock::default();
    let woke = hours(0) + TimeDelta::seconds(20);
    assert_eq!(titles(&clock.due(&events, woke, &ReminderSettings::default())), ["Standup"]);
}

#[test]
fn solo_all_day_declined_and_quiet_calendar_events_do_not_remind() {
    let settings = ReminderSettings::default();
    let mut solo = meeting("Focus", 30);
    solo.attendees.truncate(1);
    let mut all_day = meeting("Offsite", 30);
    all_day.is_all_day = true;
    let mut declined = meeting("Skip", 30);
    declined.my_rsvp = Some(Rsvp::Declined);
    for event in [&solo, &all_day, &declined] {
        assert!(!wants_reminder(event, &settings), "{}", event.title);
    }

    let mut quiet = meeting("Family", 30);
    let family = CalendarId::generate(&FakeClock::new(deino_testkit::at(NOW)));
    quiet.calendar_id = Some(family);
    let mut muted = settings.clone();
    muted.quiet_calendars.insert(family);
    assert!(wants_reminder(&quiet, &settings));
    assert!(!wants_reminder(&quiet, &muted));
}
