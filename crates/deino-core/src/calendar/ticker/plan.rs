//! [`TickState`]: what one tick of the meeting ticker does, as pure
//! bookkeeping. Which reminder window to show or close, and which event
//! whose note is open has just started. The ticker reads the inputs and
//! carries out the [`TickPlan`].

use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::CalendarEventId;

use crate::calendar::CalendarEvent;
use crate::calendar::notify::ReminderClock;
use crate::settings::ReminderSettings;

/// How long the reminder window stays up unless answered. It shows at most
/// a minute ahead, so it is still there when the meeting starts.
pub const REMINDER_OPEN_FOR: TimeDelta = TimeDelta::seconds(60);

/// How late after an event's start opening its note still starts recording.
/// Past it, the user opened an old note, not a meeting about to happen.
pub const START_GRACE: TimeDelta = TimeDelta::minutes(1);

/// What one tick sees.
#[derive(Clone, Copy, Debug)]
pub struct TickInput<'a> {
    /// The events around now.
    pub events: &'a [CalendarEvent],
    /// The tick's time.
    pub now: DateTime<Utc>,
    /// Which calendars remind.
    pub reminders: &'a ReminderSettings,
    /// Deino's Do Not Disturb is on: reminders are skipped, not postponed.
    pub muted: bool,
    /// The event whose note the user has open, if any.
    pub open_note: Option<CalendarEventId>,
}

/// What one tick does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TickPlan {
    /// Show the reminder window for this event (replacing any other).
    pub show: Option<CalendarEvent>,
    /// Close the reminder window.
    pub close: bool,
    /// The event whose note is open and which has just started.
    pub start: Option<CalendarEvent>,
}

/// The reminder window on screen.
#[derive(Clone, Copy, Debug)]
struct ShownWindow {
    close_at: DateTime<Utc>,
}

/// What the ticker remembers between ticks.
#[derive(Debug, Default)]
pub struct TickState {
    reminders: ReminderClock,
    window: Option<ShownWindow>,
    started: BTreeSet<CalendarEventId>,
}

impl TickState {
    /// The plan for one tick over `input`.
    pub fn plan(&mut self, input: &TickInput<'_>) -> TickPlan {
        let now = input.now;
        let expired = self.window.take_if(|window| window.close_at <= now);
        // Every due event counts as reminded, muted or not, so turning Do
        // Not Disturb off never shows a reminder for a meeting under way.
        let due = self.reminders.due(input.events, now, input.reminders);
        let show = due
            .into_iter()
            .filter(|_| !input.muted)
            .min_by_key(|event| event.start_at)
            .cloned();
        if show.is_some() {
            let close_at = now + REMINDER_OPEN_FOR;
            self.window = Some(ShownWindow { close_at });
        }
        TickPlan {
            close: expired.is_some() && show.is_none(),
            start: self.started_now(input),
            show,
        }
    }

    /// The open note's event, once, when it started within [`START_GRACE`].
    fn started_now(&mut self, input: &TickInput<'_>) -> Option<CalendarEvent> {
        let now = input.now;
        self.started.retain(|id| {
            input
                .events
                .iter()
                .any(|event| event.id == *id && event.end_at > now)
        });
        let id = input.open_note?;
        let event = input.events.iter().find(|event| event.id == id)?;
        let just_started = event.start_at <= now && now - event.start_at < START_GRACE;
        (just_started && self.started.insert(id)).then(|| event.clone())
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
