//! Each track's capture state for the whole recording, shared lock-free
//! with whoever watches it: the live pipeline (should it wait for a lagging
//! track, or pad it with silence?), recording health (has a track gone
//! quiet?) and the record panel's level meter.
//!
//! A track's [`TrackMeter`] outlives every source behind it (a pause, a
//! device switch, a recovery), so its frame count only ever grows and its
//! level follows whatever device feeds it now. The [`TrackStatus`] is
//! written by the capture supervisor on every tick and every change.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use super::meter::TrackMeter;
use crate::units::Track;

/// What a track's capture is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackStatus {
    /// A source runs and delivers audio.
    #[default]
    Live,
    /// The source failed or stalled; the supervisor is restarting it.
    Recovering,
    /// No device can feed the track (no permission, unplugged, given up);
    /// it records silence until a later retry succeeds.
    Absent,
    /// Stopped on purpose: paused, asleep, or switched off by the user.
    Off,
}

impl TrackStatus {
    /// Whether audio is expected from the track now. A track that is not
    /// expected to deliver can be padded with silence by its readers; one
    /// that is merely late must be waited for.
    pub const fn delivers(self) -> bool {
        matches!(self, Self::Live)
    }

    const fn to_bits(self) -> u8 {
        match self {
            Self::Live => 0,
            Self::Recovering => 1,
            Self::Absent => 2,
            Self::Off => 3,
        }
    }

    const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Live,
            1 => Self::Recovering,
            2 => Self::Absent,
            _ => Self::Off,
        }
    }
}

/// One track's shared capture state.
#[derive(Debug, Default)]
pub struct TrackState {
    meter: Arc<TrackMeter>,
    status: AtomicU8,
}

impl TrackState {
    /// The track's activity counters, kept across source restarts.
    pub fn meter(&self) -> &Arc<TrackMeter> {
        &self.meter
    }

    /// What the track's capture is doing now.
    pub fn status(&self) -> TrackStatus {
        TrackStatus::from_bits(self.status.load(Ordering::Acquire))
    }

    /// Records what the track's capture is doing (the supervisor only).
    pub fn set_status(&self, status: TrackStatus) {
        self.status.store(status.to_bits(), Ordering::Release);
    }
}

/// Both tracks' shared capture state.
#[derive(Debug, Default)]
pub struct TrackStates {
    microphone: TrackState,
    system: TrackState,
}

impl TrackStates {
    /// The state of `track`.
    pub fn get(&self, track: Track) -> &TrackState {
        match track {
            Track::Microphone => &self.microphone,
            Track::System => &self.system,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Frames, HostTime};

    #[test]
    fn a_status_round_trips_through_its_atomic() {
        let state = TrackState::default();
        assert_eq!(state.status(), TrackStatus::Live);
        for status in [
            TrackStatus::Recovering,
            TrackStatus::Absent,
            TrackStatus::Off,
            TrackStatus::Live,
        ] {
            state.set_status(status);
            assert_eq!(state.status(), status);
        }
    }

    #[test]
    fn only_a_live_track_is_expected_to_deliver() {
        assert!(TrackStatus::Live.delivers());
        assert!(!TrackStatus::Recovering.delivers());
        assert!(!TrackStatus::Absent.delivers());
        assert!(!TrackStatus::Off.delivers());
    }

    #[test]
    fn each_track_has_its_own_meter() {
        let states = TrackStates::default();
        states
            .get(Track::System)
            .meter()
            .record(480, HostTime(1), 0.5);
        assert_eq!(states.get(Track::System).meter().frames(), Frames(480));
        assert_eq!(states.get(Track::Microphone).meter().frames(), Frames(0));
    }
}
