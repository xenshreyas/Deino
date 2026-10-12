//! The tick plan: one window at a time, closed after a minute, nothing
//! while muted, and one start per open note.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::CalendarEventId;

use super::{TickInput, TickPlan, TickState};
use crate::calendar::CalendarEvent;
use crate::calendar::test_events::{hours, stored};
use crate::settings::ReminderSettings;

/// A two-person meeting starting `seconds` after the fixture's now.
fn meeting(title: &str, seconds: i64) -> CalendarEvent {
    let mut event = stored(title, 0);
    event.start_at = hours(0) + TimeDelta::seconds(seconds);
    event.end_at = event.start_at + TimeDelta::minutes(30);
    event.attendees.push(event.attendees[0].clone());
    event
}

fn at(seconds: i64) -> DateTime<Utc> {
    hours(0) + TimeDelta::seconds(seconds)
}

struct Ticks {
    state: TickState,
    events: Vec<CalendarEvent>,
    reminders: ReminderSettings,
    muted: bool,
    open_note: Option<CalendarEventId>,
}

impl Ticks {
    fn new(events: Vec<CalendarEvent>) -> Self {
        Self {
            state: TickState::default(),
            events,
            reminders: ReminderSettings::default(),
            muted: false,
            open_note: None,
        }
    }

    fn at(&mut self, seconds: i64) -> TickPlan {
        self.state.plan(&TickInput {
            events: &self.events,
            now: at(seconds),
            reminders: &self.reminders,
            muted: self.muted,
            open_note: self.open_note,
        })
    }
}

fn shown(plan: &TickPlan) -> Option<&str> {
    plan.show.as_ref().map(|event| event.title.as_str())
}

#[test]
fn the_window_shows_once_and_closes_after_a_minute() {
    let mut ticks = Ticks::new(vec![meeting("standup", 45)]);
    assert!(ticks.at(-30).show.is_none(), "more than a minute ahead");
    assert_eq!(shown(&ticks.at(0)), Some("standup"));
    let later = ticks.at(15);
    assert_eq!((shown(&later), later.close), (None, false));
    assert!(!ticks.at(45).close, "still up as the meeting starts");
    assert!(ticks.at(60).close);
    assert!(!ticks.at(75).close, "closed once");
}

#[test]
fn of_two_meetings_due_together_the_sooner_shows() {
    let mut ticks = Ticks::new(vec![meeting("later", 50), meeting("sooner", 20)]);
    assert_eq!(shown(&ticks.at(0)), Some("sooner"));
}

#[test]
fn a_new_reminder_replaces_the_window_instead_of_closing_it() {
    let mut ticks = Ticks::new(vec![meeting("first", 10), meeting("second", 70)]);
    assert_eq!(shown(&ticks.at(0)), Some("first"));
    let next = ticks.at(15);
    assert_eq!((shown(&next), next.close), (Some("second"), false));
}

#[test]
fn muted_reminders_are_skipped_not_postponed() {
    let mut ticks = Ticks::new(vec![meeting("standup", 30)]);
    ticks.muted = true;
    assert_eq!(ticks.at(0), TickPlan::default());
    ticks.muted = false;
    assert!(ticks.at(15).show.is_none());
}

#[test]
fn quiet_calendars_never_remind() {
    let mut ticks = Ticks::new(vec![meeting("standup", 30)]);
    ticks.reminders.enabled = false;
    assert!(ticks.at(0).show.is_none());
}

#[test]
fn an_open_note_starts_its_meeting_once_at_the_start() {
    let standup = meeting("standup", 30);
    let mut ticks = Ticks::new(vec![standup.clone()]);
    ticks.open_note = Some(standup.id);
    assert!(ticks.at(15).start.is_none(), "not yet");
    assert_eq!(ticks.at(30).start.map(|event| event.id), Some(standup.id));
    assert!(ticks.at(45).start.is_none(), "once");
}

#[test]
fn a_note_opened_long_after_the_start_does_not_start_it() {
    let standup = meeting("standup", 0);
    let mut ticks = Ticks::new(vec![standup.clone()]);
    ticks.open_note = Some(standup.id);
    assert!(ticks.at(60).start.is_none());
}

#[test]
fn without_an_open_note_nothing_starts() {
    let mut ticks = Ticks::new(vec![meeting("standup", 0)]);
    assert!(ticks.at(5).start.is_none());
}
