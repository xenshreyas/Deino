//! One track's slot in the [`super::tracks::TrackSet`]: what runs, its
//! watchdog, whether the user wants it, and the status readers see.

use crate::capture::state::TrackStatus;
use crate::capture::track::TrackCapture;
use crate::capture::watchdog::{TrackSample, TrackWatch, WatchAction, WatchPolicy};
use crate::ring::RtSink;
use crate::units::{Frames, HostTime, Track};

/// A track that is capturing, or waiting for its device.
#[derive(Debug)]
pub(super) enum Slot {
    Running(TrackCapture),
    Absent(RtSink),
    /// Momentarily empty while a start is attempted.
    Empty,
}

/// One track: its slot, its watchdog and whether the user wants it.
#[derive(Debug)]
pub(super) struct Watched {
    pub(super) slot: Slot,
    pub(super) watch: TrackWatch,
    pub(super) enabled: bool,
}

/// A fresh, wanted track in `slot`, considered healthy at `now`.
pub(super) fn watched(track: Track, slot: Slot, now: HostTime) -> Watched {
    let policy = match track {
        Track::Microphone => WatchPolicy::MICROPHONE,
        Track::System => WatchPolicy::SYSTEM,
    };
    Watched {
        slot,
        watch: TrackWatch::new(policy, now),
        enabled: true,
    }
}

/// What a track's capture is doing, as its readers see it.
pub(super) fn status_of(watched: &Watched, suspended: bool) -> TrackStatus {
    if suspended || !watched.enabled {
        return TrackStatus::Off;
    }
    match &watched.slot {
        Slot::Running(_) if watched.watch.is_recovering() => TrackStatus::Recovering,
        Slot::Running(capture) if capture.is_running() => TrackStatus::Live,
        Slot::Running(_) => TrackStatus::Off,
        Slot::Absent(_) | Slot::Empty => TrackStatus::Absent,
    }
}

/// Feeds one tick of `watched`'s activity to its watchdog. A suspended or
/// switched-off track is left alone; an absent one counts as a dead device.
pub(super) fn observe(watched: &mut Watched, now: HostTime) -> WatchAction {
    if !watched.enabled {
        return WatchAction::None;
    }
    let sample = match &watched.slot {
        Slot::Running(capture) if !capture.is_running() => return WatchAction::None,
        Slot::Running(capture) => TrackSample {
            frames: capture.meter().frames(),
            alive: capture.is_alive(),
        },
        Slot::Absent(_) | Slot::Empty => TrackSample {
            frames: Frames::ZERO,
            alive: false,
        },
    };
    watched.watch.observe(now, sample)
}
