//! Deciding when a refusing encoder counts as stalled.

use std::time::Duration;

use crate::health::HealthEvent;
use crate::units::{HostTime, Track};

/// How long the encoder may refuse audio before [`HealthEvent::Stalled`]:
/// two segments' worth, longer than any normal encoder hiccup.
pub const STALL_THRESHOLD: Duration = Duration::from_secs(2);

/// Tracks how long a track's encoder has been refusing queued audio.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StallWatch {
    track: Track,
    waiting_since: Option<HostTime>,
    reported: bool,
}

impl StallWatch {
    /// A watch for `track` that has seen no refusal.
    pub(crate) const fn new(track: Track) -> Self {
        Self {
            track,
            waiting_since: None,
            reported: false,
        }
    }

    /// How long audio has been waiting at `now` (zero when it is not).
    pub(crate) fn waiting_for(&self, now: HostTime) -> Duration {
        self.waiting_since
            .map_or(Duration::ZERO, |since| now.since(since))
    }

    /// Records whether audio is `waiting` (queued while the encoder refuses
    /// it) at `now`, and returns the event to raise, if any: `Stalled` once
    /// past [`STALL_THRESHOLD`], then `Resumed` when audio flows again.
    pub(crate) fn observe(&mut self, now: HostTime, waiting: bool) -> Option<HealthEvent> {
        if !waiting {
            self.waiting_since = None;
            let resumed = std::mem::replace(&mut self.reported, false);
            return resumed.then_some(HealthEvent::Resumed { track: self.track });
        }
        let since = *self.waiting_since.get_or_insert(now);
        let waited = now.since(since);
        if self.reported || waited < STALL_THRESHOLD {
            return None;
        }
        self.reported = true;
        Some(HealthEvent::Stalled {
            track: self.track,
            for_at_least: waited,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(millis: u64) -> HostTime {
        HostTime(millis * 1_000_000)
    }

    #[test]
    fn a_short_refusal_is_not_a_stall() {
        let mut watch = StallWatch::new(Track::System);
        assert_eq!(watch.observe(at(0), true), None);
        assert_eq!(watch.observe(at(1_999), true), None);
        assert_eq!(watch.observe(at(2_005), false), None);
    }

    #[test]
    fn a_long_refusal_is_reported_once_then_resumed() {
        let mut watch = StallWatch::new(Track::Microphone);
        watch.observe(at(100), true);

        let stalled = watch.observe(at(2_100), true);
        let repeated = watch.observe(at(5_000), true);
        let resumed = watch.observe(at(5_010), false);

        assert_eq!(
            stalled,
            Some(HealthEvent::Stalled {
                track: Track::Microphone,
                for_at_least: STALL_THRESHOLD,
            })
        );
        assert_eq!(repeated, None);
        assert_eq!(
            resumed,
            Some(HealthEvent::Resumed {
                track: Track::Microphone
            })
        );
        assert_eq!(watch.waiting_for(at(6_000)), Duration::ZERO);
    }
}
