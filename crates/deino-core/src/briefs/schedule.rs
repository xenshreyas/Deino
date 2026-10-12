//! When a brief is due, and for which events (pure rules over the clock and
//! the user's time zone).
//!
//! Two runs per event: the morning run from 06:00 local time covers the
//! day's remaining events (Granola writes briefs overnight), and a run an
//! hour before each event catches what changed since. A Mac asleep at 06:00
//! catches up on its first tick after waking; nothing runs once an event has
//! started.

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use deino_domain::{LocalZone, Rsvp};

use crate::calendar::CalendarEvent;
use crate::settings::BriefAudience;

/// When the morning run starts, local time.
pub const MORNING_RUN: NaiveTime = match NaiveTime::from_hms_opt(6, 0, 0) {
    Some(time) => time,
    None => NaiveTime::MIN,
};

/// How long before an event its second run is due.
pub const LEAD_TIME: TimeDelta = TimeDelta::minutes(60);

/// The fewest attendees a meeting needs under [`BriefAudience::Groups`].
pub const MIN_GROUP_ATTENDEES: usize = 2;

/// Which run is due.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BriefTrigger {
    /// The 06:00 run, for the rest of the day.
    Morning,
    /// The run an hour before the event.
    BeforeEvent,
}

/// The run due for an event starting at `start`, at `now`; the hour before
/// the event wins over the morning.
pub fn due_trigger(
    start: DateTime<Utc>,
    now: DateTime<Utc>,
    zone: &dyn LocalZone,
) -> Option<BriefTrigger> {
    if now >= start {
        return None;
    }
    if start - now <= LEAD_TIME {
        return Some(BriefTrigger::BeforeEvent);
    }
    let local_now = zone.local(now);
    let same_day = zone.local(start).date_naive() == local_now.date_naive();
    (same_day && local_now.time() >= MORNING_RUN).then_some(BriefTrigger::Morning)
}

/// The span of starts a tick at `now` may find due: until the end of the
/// local day, or the next hour when that reaches further.
pub fn due_window(now: DateTime<Utc>, zone: &dyn LocalZone) -> (DateTime<Utc>, DateTime<Utc>) {
    let local = zone.local(now);
    let midnight = local
        .date_naive()
        .succ_opt()
        .and_then(|day| {
            day.and_time(NaiveTime::MIN)
                .and_local_timezone(*local.offset())
                .single()
        })
        .map_or(now, |end| end.with_timezone(&Utc));
    (now, midnight.max(now + LEAD_TIME))
}

/// Whether `event` gets a brief under `audience`: a timed meeting the user
/// has not declined, with someone external (or, for groups, two or more
/// attendees).
pub fn qualifies(event: &CalendarEvent, audience: BriefAudience) -> bool {
    if event.is_all_day || event.my_rsvp == Some(Rsvp::Declined) {
        return false;
    }
    match audience {
        BriefAudience::External => event.is_external,
        BriefAudience::Groups => event.attendees.len() >= MIN_GROUP_ATTENDEES,
    }
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;
