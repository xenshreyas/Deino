//! Meeting reminders (ORG-02): which events show the "starting in 1
//! minute" window with Join & record and Record only (Granola §2.2).
//!
//! [`ReminderClock`] is pure bookkeeping the app's ticker drives: each tick
//! hands it the events around now ([`CalendarService::events_near`](super::CalendarService::events_near))
//! and the time, and it returns the meetings starting within
//! [`REMINDER_LEAD`] that have not reminded yet. Each event reminds once,
//! even when a resync moves it by a few seconds, and a late tick (the Mac
//! slept) still reminds for meetings that have not started.

use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{CalendarEventId, Rsvp};

use super::CalendarEvent;
use super::rules::MIN_MEETING_ATTENDEES;
use crate::settings::ReminderSettings;

/// How long before an event its reminder shows.
pub const REMINDER_LEAD: TimeDelta = TimeDelta::minutes(1);

/// True when `event` is a meeting worth a reminder: not all day, with at
/// least [`MIN_MEETING_ATTENDEES`] people, not declined, on a calendar the
/// user did not make quiet.
pub fn wants_reminder(event: &CalendarEvent, settings: &ReminderSettings) -> bool {
    !event.is_all_day
        && event.attendees.len() >= MIN_MEETING_ATTENDEES
        && event.my_rsvp != Some(Rsvp::Declined)
        && settings.reminds_for(event.calendar_id.as_ref())
}

/// Remembers which events already reminded.
#[derive(Debug, Default)]
pub struct ReminderClock {
    shown: BTreeSet<CalendarEventId>,
}

impl ReminderClock {
    /// The events (of `events`) whose reminder is due at `now`.
    pub fn due<'a>(
        &mut self,
        events: &'a [CalendarEvent],
        now: DateTime<Utc>,
        settings: &ReminderSettings,
    ) -> Vec<&'a CalendarEvent> {
        let due: Vec<&CalendarEvent> = events
            .iter()
            .filter(|event| starts_soon(event, now) && wants_reminder(event, settings))
            .filter(|event| !self.shown.contains(&event.id))
            .collect();
        self.shown.extend(due.iter().map(|event| event.id));
        self.forget_past(events, now);
        due
    }

    /// Drops remembered events that have ended or left the list, so the
    /// set stays as small as the window of events around now.
    fn forget_past(&mut self, events: &[CalendarEvent], now: DateTime<Utc>) {
        let live: BTreeSet<CalendarEventId> = events
            .iter()
            .filter(|event| event.end_at > now)
            .map(|event| event.id)
            .collect();
        self.shown.retain(|id| live.contains(id));
    }
}

/// True when `event` starts after `now` and at most [`REMINDER_LEAD`] later.
fn starts_soon(event: &CalendarEvent, now: DateTime<Utc>) -> bool {
    event.start_at > now && event.start_at - now <= REMINDER_LEAD
}

#[cfg(test)]
mod notify_tests;
