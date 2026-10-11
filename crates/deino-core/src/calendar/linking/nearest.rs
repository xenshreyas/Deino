//! The linking rules as pure functions: which event a meeting starting now
//! belongs to, and how the picker orders its suggestions.

use std::cmp::Reverse;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::Rsvp;

use crate::calendar::{CalendarEvent, TimeRange};

/// How far ahead an event may start and still be the one a meeting
/// starting now belongs to (the card's "within 15 minutes").
pub const LINK_WINDOW: TimeDelta = TimeDelta::minutes(15);

/// How far before a meeting the picker looks for events (Granola's
/// "events from the past 24 hours").
pub const SUGGEST_BEFORE: TimeDelta = TimeDelta::hours(24);

/// How far after a meeting's start the picker still suggests events, for a
/// recording started early.
pub const SUGGEST_AFTER: TimeDelta = TimeDelta::hours(1);

/// The most events the picker lists.
pub const MAX_SUGGESTIONS: usize = 20;

/// The end of a [`TimeRange`] is exclusive; this makes an event starting
/// exactly [`LINK_WINDOW`] from now part of [`link_range`].
const INCLUSIVE: TimeDelta = TimeDelta::seconds(1);

/// True for an event a meeting can be linked to: one with a time (not an
/// all-day block) that the user did not decline.
pub fn is_linkable(event: &CalendarEvent) -> bool {
    !event.is_all_day && event.my_rsvp != Some(Rsvp::Declined)
}

/// True when `event` is under way at `now` or starts within [`LINK_WINDOW`].
fn is_near(event: &CalendarEvent, now: DateTime<Utc>) -> bool {
    event.start_at - LINK_WINDOW <= now && now < event.end_at
}

/// The event a meeting starting at `now` belongs to: of the linkable events
/// under way or starting within [`LINK_WINDOW`], the one whose start is
/// closest to `now`. Back to back, the next meeting wins once it is closer
/// than the start of the one ending; on a tie the one under way wins.
pub fn nearest_event(events: &[CalendarEvent], now: DateTime<Utc>) -> Option<&CalendarEvent> {
    events
        .iter()
        .filter(|event| is_linkable(event) && is_near(event, now))
        .min_by_key(|event| ((event.start_at - now).abs(), event.start_at > now, event.id))
}

/// The span [`nearest_event`] looks at: events overlapping it are the only
/// ones that can be under way at `now` or start within [`LINK_WINDOW`].
pub fn link_range(now: DateTime<Utc>) -> TimeRange {
    TimeRange {
        start: now,
        end: now + LINK_WINDOW + INCLUSIVE,
    }
}

/// The span the picker suggests events from, around a meeting that started
/// (or was created) at `anchor`.
pub fn suggestion_range(anchor: DateTime<Utc>) -> TimeRange {
    TimeRange {
        start: anchor - SUGGEST_BEFORE,
        end: anchor + SUGGEST_AFTER,
    }
}

/// The picker's suggestions: the linkable events, the start closest to
/// `anchor` first (later first on a tie), at most [`MAX_SUGGESTIONS`].
pub fn rank_suggestions(events: Vec<CalendarEvent>, anchor: DateTime<Utc>) -> Vec<CalendarEvent> {
    let mut linkable: Vec<CalendarEvent> = events.into_iter().filter(is_linkable).collect();
    linkable.sort_by_key(|event| {
        (
            (event.start_at - anchor).abs(),
            Reverse(event.start_at),
            event.id,
        )
    });
    linkable.truncate(MAX_SUGGESTIONS);
    linkable
}
