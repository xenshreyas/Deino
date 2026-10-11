//! Calendar association: a detected call that starts near a calendar event
//! is assumed to be that event (Granola §2.3), so the prompt names it.
//!
//! Which event a call belongs to is the calendar's rule, the same one that
//! links a recording when it starts (ORG-03,
//! `deino_core::calendar::linking::nearest_event`: under way or starting
//! within 15 minutes, the closest start first, never all-day or declined).
//! The app applies it over the synced calendar
//! (`features::calendar::SyncedCalendar`); [`NoCalendar`] is for tests and
//! builds without calendars.

use chrono::{DateTime, Utc};
use deino_domain::CalendarEventId;
use serde::Serialize;

/// A calendar event a detection is linked to. The title is meeting content:
/// shown in the prompt, never logged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEvent {
    /// The event.
    pub id: CalendarEventId,
    /// Its title.
    pub title: String,
    /// When it starts.
    pub starts_at: DateTime<Utc>,
}

/// Finds the calendar event a call starting `now` belongs to.
pub trait CalendarLookup: Send + Sync {
    /// The event a call starting at `now` belongs to, if any.
    fn linked_event(&self, now: DateTime<Utc>) -> Option<LinkedEvent>;
}

/// The lookup before calendars exist: never links an event.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoCalendar;

impl CalendarLookup for NoCalendar {
    fn linked_event(&self, _now: DateTime<Utc>) -> Option<LinkedEvent> {
        None
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn without_calendars_nothing_links() {
        let now = Utc.with_ymd_and_hms(2026, 10, 9, 10, 0, 0).unwrap();

        assert_eq!(NoCalendar.linked_event(now), None);
    }
}
