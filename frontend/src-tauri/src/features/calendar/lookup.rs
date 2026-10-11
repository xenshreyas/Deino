//! [`SyncedCalendar`]: CAP-06's [`CalendarLookup`] over the synced events,
//! so a detected call is linked to the calendar event it belongs to.
//!
//! The detector asks synchronously from its own thread, so the lookup reads
//! [`CalendarService::events_near`] (kept in memory by every sync) and never
//! the database. The service is found in Tauri state at each call: detection
//! starts before the database, and links nothing until calendars exist.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use deino_core::calendar::{CalendarEvent, CalendarService};
use deino_detect::calendar::pick_linked;
use deino_detect::{CalendarLookup, LinkedEvent};
use deino_domain::Rsvp;
use tauri::{AppHandle, Manager, Runtime};

/// The synced calendar, as meeting detection sees it.
pub struct SyncedCalendar<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> SyncedCalendar<R> {
    /// A lookup over the `Arc<CalendarService>` that `app` manages, once it does.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> CalendarLookup for SyncedCalendar<R> {
    fn linked_event(&self, now: DateTime<Utc>) -> Option<LinkedEvent> {
        let calendar = self.app.try_state::<Arc<CalendarService>>()?;
        linked_event(&calendar.events_near(now), now)
    }
}

/// The event among `events` a call at `now` belongs to: never an all-day
/// event or one the user declined (CAP-06 card), else the start closest to
/// `now` within the link window.
pub(crate) fn linked_event(events: &[CalendarEvent], now: DateTime<Utc>) -> Option<LinkedEvent> {
    let candidates = events
        .iter()
        .filter(|event| !event.is_all_day && event.my_rsvp != Some(Rsvp::Declined))
        .map(|event| LinkedEvent {
            id: event.id,
            title: event.title.clone(),
            starts_at: event.start_at,
        });
    pick_linked(candidates, now)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone};
    use deino_domain::{CalendarAccountId, CalendarEventId};

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 9, 10, 0, 0).unwrap()
    }

    fn id(last: u8) -> CalendarEventId {
        format!("01926b2c-0000-7000-8000-0000000000{last:02}")
            .parse()
            .unwrap()
    }

    /// A 30-minute event `last` starting `minutes` from now.
    fn event(last: u8, minutes: i64) -> CalendarEvent {
        let start_at = now() + TimeDelta::minutes(minutes);
        CalendarEvent {
            id: id(last),
            account_id: "01926b2c-0000-7000-8000-000000000a01"
                .parse::<CalendarAccountId>()
                .unwrap(),
            calendar_id: None,
            provider_event_id: format!("event-{last}"),
            ical_uid: None,
            series_id: None,
            title: format!("Event {last}"),
            description: String::new(),
            start_at,
            end_at: start_at + TimeDelta::minutes(30),
            is_all_day: false,
            conference_url: None,
            platform: None,
            organizer_email: None,
            attendees: Vec::new(),
            my_rsvp: Some(Rsvp::Accepted),
            is_external: false,
            record_override: None,
            created_at: now(),
            updated_at: now(),
        }
    }

    #[test]
    fn a_call_links_to_the_event_starting_closest_to_now() {
        let events = [event(1, -10), event(2, 3), event(3, 40)];

        let linked = linked_event(&events, now()).unwrap();

        assert_eq!(linked.id, id(2));
        assert_eq!(linked.title, "Event 2");
        assert_eq!(linked.starts_at, now() + TimeDelta::minutes(3));
    }

    #[test]
    fn all_day_and_declined_events_never_link() {
        let all_day = CalendarEvent {
            is_all_day: true,
            ..event(1, 0)
        };
        let declined = CalendarEvent {
            my_rsvp: Some(Rsvp::Declined),
            ..event(2, 1)
        };

        assert_eq!(linked_event(&[all_day, declined], now()), None);
    }

    #[test]
    fn events_outside_the_link_window_do_not_link() {
        assert_eq!(linked_event(&[event(1, -20), event(2, 6)], now()), None);
    }
}
