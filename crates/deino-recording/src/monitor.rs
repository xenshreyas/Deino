//! Recording health (CAP-10): the silent failures a user cannot see until
//! the meeting is over, caught while it runs.
//!
//! [`HealthMonitor`] is pure. Once a second the session's watcher thread
//! ([`watch`]) gathers [`Vitals`] (frame counters, track states, lane
//! activity, the engines' stream status, free disk) and the monitor turns
//! them into the set of open [`HealthIssue`]s, raising and clearing each
//! one (`audio-capture.md` §7; muesli's mic health tracker and Cap's health
//! bus, reimplemented):
//!
//! | Issue | Raised when | Cleared when |
//! |---|---|---|
//! | `noAudio` (per track) | a track that should run delivered no frames for [`NO_AUDIO_AFTER`] | frames flow again, or the track is switched off |
//! | `noTranscript` | the microphone heard [`NO_TRANSCRIPT_AFTER`] of speech without any Me text | Me text arrives |
//! | `transcriptionStopped` (per lane) | the lane's engine lost its stream (a worker crash or hang) | the stream has run again for [`ENGINE_SETTLE`] |
//! | `diskLow` | free space under [`DISK_LOW`] | free space over it plus [`DISK_MARGIN`] |
//! | `diskFull` | free space under [`DISK_FULL`]: the recorder stops cleanly | (the recording ends) |

pub(crate) mod watch;

use std::time::Duration;

use deino_audio::capture::TrackStatus;
use deino_audio::{Bytes, Frames, HostTime};
use deino_domain::Channel;
use deino_stt::{LaneStatus, Millis};
use serde::Serialize;

use crate::lane::{Lane, PerLane};

/// A track that should run but delivers nothing for this long is silent.
pub const NO_AUDIO_AFTER: Duration = Duration::from_secs(5);

/// Speech on the microphone this long without a word of transcript.
pub const NO_TRANSCRIPT_AFTER: Millis = Millis(30_000);

/// How long a lane's engine must stream again before its warning clears.
pub const ENGINE_SETTLE: Duration = Duration::from_secs(10);

/// Free space under which the disk is low (CAP-10's 2 GB).
pub const DISK_LOW: Bytes = deino_audio::health::DISK_LOW;

/// Free space under which recording stops cleanly before the disk fills
/// (`audio-capture.md` §5: 256 MiB).
pub const DISK_FULL: Bytes = Bytes(256 * 1024 * 1024);

/// Hysteresis above [`DISK_LOW`] before the warning clears.
pub const DISK_MARGIN: Bytes = Bytes(256 * 1024 * 1024);

/// What can be wrong with a running recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum IssueKind {
    /// A track delivers no audio.
    NoAudio,
    /// The microphone hears speech but nothing is transcribed.
    NoTranscript,
    /// A lane's transcription engine stopped (crashed or hung) and is
    /// recovering.
    TranscriptionStopped,
    /// The disk is nearly full.
    DiskLow,
    /// The disk is full: recording stops.
    DiskFull,
}

/// One open problem, on a channel when it concerns one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HealthIssue {
    /// What is wrong.
    pub kind: IssueKind,
    /// The track or lane it concerns (Me = `mic`, Them = `system`).
    pub channel: Option<Channel>,
}

/// One track's capture, as the monitor sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackVitals {
    /// Frames delivered so far.
    pub frames: Frames,
    /// What its capture is doing.
    pub status: TrackStatus,
}

/// Everything the monitor looks at, once a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vitals {
    /// Each lane's track.
    pub tracks: PerLane<TrackVitals>,
    /// Speech the microphone heard since the Me lane last produced text.
    pub unanswered: Millis,
    /// Each lane's engine (`None` for engines that do not stream).
    pub engines: PerLane<Option<LaneStatus>>,
    /// Free space on the recording's volume, when it could be read.
    pub free_disk: Option<Bytes>,
}

/// What the monitor remembers about one track.
#[derive(Clone, Copy, Debug)]
struct TrackMemory {
    frames: Frames,
    changed_at: HostTime,
}

/// What the monitor remembers about one lane's engine.
#[derive(Clone, Copy, Debug, Default)]
struct EngineMemory {
    recoveries: u32,
    /// When the engine last lost its stream (or was seen not streaming).
    troubled_at: Option<HostTime>,
}

/// The open issues and what they were judged from.
#[derive(Debug)]
pub struct HealthMonitor {
    tracks: PerLane<TrackMemory>,
    engines: PerLane<EngineMemory>,
    issues: Vec<HealthIssue>,
}

impl HealthMonitor {
    /// A monitor for a recording that started at `now`.
    pub fn new(now: HostTime) -> Self {
        let track = TrackMemory {
            frames: Frames::ZERO,
            changed_at: now,
        };
        Self {
            tracks: PerLane {
                me: track,
                them: track,
            },
            engines: PerLane::default(),
            issues: Vec::new(),
        }
    }

    /// The open issues, in a stable order.
    pub fn issues(&self) -> &[HealthIssue] {
        &self.issues
    }

    /// Judges `vitals` at `now`; returns whether the open issues changed.
    pub fn evaluate(&mut self, vitals: &Vitals, now: HostTime) -> bool {
        let mut open = Vec::new();
        for lane in Lane::ALL {
            let channel = Some(lane.channel());
            if self.silent(lane, vitals.tracks.get(lane), now) {
                open.push(issue(IssueKind::NoAudio, channel));
            }
            if self.engine_down(lane, *vitals.engines.get(lane), now) {
                open.push(issue(IssueKind::TranscriptionStopped, channel));
            }
        }
        if vitals.unanswered >= NO_TRANSCRIPT_AFTER {
            open.push(issue(IssueKind::NoTranscript, Some(Channel::Mic)));
        }
        open.extend(self.disk(vitals.free_disk));
        open.sort_unstable();
        let changed = open != self.issues;
        self.issues = open;
        changed
    }

    /// Whether `lane`'s track has been silent too long while it should run.
    fn silent(&mut self, lane: Lane, track: &TrackVitals, now: HostTime) -> bool {
        let memory = self.tracks.get_mut(lane);
        if track.status == TrackStatus::Off || track.frames != memory.frames {
            memory.frames = track.frames;
            memory.changed_at = now;
        }
        now.since(memory.changed_at) >= NO_AUDIO_AFTER
    }

    /// Whether `lane`'s engine lost its stream recently or has none now.
    fn engine_down(&mut self, lane: Lane, status: Option<LaneStatus>, now: HostTime) -> bool {
        let Some(status) = status else {
            return false;
        };
        let memory = self.engines.get_mut(lane);
        if status.recoveries > memory.recoveries || !status.streaming {
            memory.recoveries = status.recoveries;
            memory.troubled_at = Some(now);
        }
        memory
            .troubled_at
            .is_some_and(|at| now.since(at) < ENGINE_SETTLE)
    }

    /// The disk issue, with hysteresis on the low-space warning.
    fn disk(&self, free: Option<Bytes>) -> Option<HealthIssue> {
        let free = free?;
        if free < DISK_FULL {
            return Some(issue(IssueKind::DiskFull, None));
        }
        let was_low = self.issues.iter().any(|i| i.kind == IssueKind::DiskLow);
        let limit = if was_low {
            Bytes(DISK_LOW.0 + DISK_MARGIN.0)
        } else {
            DISK_LOW
        };
        (free < limit).then(|| issue(IssueKind::DiskLow, None))
    }
}

const fn issue(kind: IssueKind, channel: Option<Channel>) -> HealthIssue {
    HealthIssue { kind, channel }
}

#[cfg(test)]
#[path = "monitor_tests.rs"]
mod tests;
