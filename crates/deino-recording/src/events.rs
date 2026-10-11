//! What a recording tells the UI: its state, the live transcript and its
//! health. The app adapts [`RecordingEvents`] to typed Tauri events.

use chrono::{DateTime, Utc};
use deino_audio::HealthEvent;
use deino_domain::{CalendarEventId, Channel, MeetingId, Millis, SegmentId};
use serde::Serialize;

use crate::echo::EchoMode;
use crate::monitor::HealthIssue;

/// Where the recorder is in its lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RecordingPhase {
    /// Nothing is being recorded.
    Idle,
    /// Devices and engines are starting.
    Starting,
    /// Audio is being captured and transcribed.
    Recording,
    /// Capture is paused; the meeting stays open.
    Paused,
    /// The last audio and segments are being saved.
    Stopping,
}

/// The recorder's state, as the UI shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSnapshot {
    /// Where the recorder is.
    pub phase: RecordingPhase,
    /// The meeting being recorded, if any.
    pub meeting_id: Option<MeetingId>,
    /// Its title.
    pub title: Option<String>,
    /// When the recording's elapsed time counts from: when capture started,
    /// or for a resumed meeting that moment moved back by what was already
    /// recorded, so the timer goes on from the meeting's length.
    pub started_at: Option<DateTime<Utc>>,
    /// Time spent paused so far; elapsed recording time is now − started − paused.
    pub paused_ms: u64,
    /// When the current pause began, while paused.
    pub paused_at: Option<DateTime<Utc>>,
    /// Whether echo is being removed from the microphone (speakers) or not
    /// needed (headphones).
    pub echo: Option<EchoMode>,
    /// The calendar event the meeting is recorded for, if any (the menu
    /// bar's "Xm left", auto-stop's calendar rule).
    pub calendar_event_id: Option<CalendarEventId>,
    /// The microphone picked for this recording by name; `None` follows
    /// the system default.
    pub microphone: Option<String>,
    /// Whether system audio (the other side of the call) is recorded.
    pub system_audio: bool,
    /// What is wrong with the recording now (CAP-10 health), most severe
    /// first; empty when all is well.
    pub issues: Vec<HealthIssue>,
}

impl RecordingSnapshot {
    /// Nothing is being recorded.
    pub fn idle() -> Self {
        Self {
            phase: RecordingPhase::Idle,
            meeting_id: None,
            title: None,
            started_at: None,
            paused_ms: 0,
            paused_at: None,
            echo: None,
            calendar_event_id: None,
            microphone: None,
            system_audio: true,
            issues: Vec::new(),
        }
    }

    /// Whether audio is being captured or the recording is paused: the
    /// phases a meeting is open in.
    pub fn is_running(&self) -> bool {
        matches!(
            self.phase,
            RecordingPhase::Recording | RecordingPhase::Paused
        )
    }
}

/// How loud each track is, for the record panel's meter (CAP-10).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevels {
    /// The microphone's loudest sample since the last reading, in `[0, 1]`.
    pub microphone: f32,
    /// System audio's loudest sample since the last reading, in `[0, 1]`.
    pub system: f32,
}

/// What happened to a live segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SegmentUpdateKind {
    /// Still being spoken; the committed text grows.
    Open,
    /// Closed and saved.
    Final,
    /// Removed as the call's echo picked up by the microphone.
    Discarded,
}

/// A transcript line as it is being spoken.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SegmentUpdate {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The segment's ID (the same row once it is saved).
    pub segment_id: SegmentId,
    /// Me (`mic`) or Them (`system`).
    pub channel: Channel,
    /// When it starts on the recording's timeline.
    pub start: Millis,
    /// When it ends (equal to `start` while open, when the engine has no end yet).
    pub end: Millis,
    /// The committed text: final words, never rewritten.
    pub text: String,
    /// Open, final or discarded.
    pub state: SegmentUpdateKind,
}

/// Something about the recording the user may need to know.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum HealthKind {
    /// A track's device failed or stalled and is being restarted.
    Recovering,
    /// A recovering track works again.
    Recovered,
    /// A track's device is gone; it records silence.
    DeviceLost,
    /// Audio was lost to a full buffer.
    FramesDropped,
    /// Saving audio is slow.
    StorageStalled,
    /// Saving audio failed.
    StorageFailed,
    /// The disk is nearly full.
    DiskLow,
    /// The microphone is a Bluetooth headset in call mode.
    BluetoothCallMode,
    /// System audio cannot be captured; only the microphone is recorded.
    SystemAudioUnavailable,
    /// The Mac slept; capture was suspended.
    Asleep,
    /// The Mac woke; capture restarted.
    Awake,
}

/// A recording health notice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HealthNotice {
    /// The meeting being recorded.
    pub meeting_id: MeetingId,
    /// What happened.
    pub kind: HealthKind,
    /// The track it concerns, when one does.
    pub channel: Option<Channel>,
}

impl HealthNotice {
    /// The notice for an audio health event, or `None` for events the UI
    /// does not show (a stall that cleared).
    pub fn from_event(meeting_id: &MeetingId, event: HealthEvent) -> Option<Self> {
        let (kind, track) = match event {
            HealthEvent::Stalled { track, .. } => (HealthKind::StorageStalled, Some(track)),
            HealthEvent::Resumed { .. } => return None,
            HealthEvent::FramesDropped { track, .. } => (HealthKind::FramesDropped, Some(track)),
            HealthEvent::DiskLow { .. } => (HealthKind::DiskLow, None),
            HealthEvent::StorageFailed { track } => (HealthKind::StorageFailed, Some(track)),
            HealthEvent::Recovering { track, .. } => (HealthKind::Recovering, Some(track)),
            HealthEvent::Recovered { track } => (HealthKind::Recovered, Some(track)),
            HealthEvent::DeviceLost { track } => (HealthKind::DeviceLost, Some(track)),
            HealthEvent::BluetoothCallMode => (HealthKind::BluetoothCallMode, None),
            HealthEvent::SystemAudioUnavailable => (HealthKind::SystemAudioUnavailable, None),
            HealthEvent::Asleep => (HealthKind::Asleep, None),
            HealthEvent::Awake => (HealthKind::Awake, None),
        };
        Some(Self {
            meeting_id: meeting_id.clone(),
            kind,
            channel: track.map(Channel::from),
        })
    }
}

/// Delivers recording events to the UI. Implementations never block and
/// never call back into the recorder (state changes are announced under
/// its lock, so they arrive in order).
pub trait RecordingEvents: Send + Sync {
    /// The recorder's state changed.
    fn state(&self, state: &RecordingSnapshot);
    /// A live segment grew, closed or was discarded.
    fn segment(&self, segment: &SegmentUpdate);
    /// A health notice.
    fn health(&self, notice: &HealthNotice);
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedEvents;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};

    use super::{HealthNotice, RecordingEvents, RecordingSnapshot, SegmentUpdate};

    /// Records every event, for tests.
    #[derive(Debug, Default)]
    pub struct RecordedEvents {
        states: Mutex<Vec<RecordingSnapshot>>,
        segments: Mutex<Vec<SegmentUpdate>>,
        health: Mutex<Vec<HealthNotice>>,
    }

    impl RecordedEvents {
        /// Every state, oldest first.
        pub fn states(&self) -> Vec<RecordingSnapshot> {
            self.states
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        /// Every live segment event, oldest first.
        pub fn segments(&self) -> Vec<SegmentUpdate> {
            self.segments
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        /// Every health notice, oldest first.
        pub fn health(&self) -> Vec<HealthNotice> {
            self.health
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    impl RecordingEvents for RecordedEvents {
        fn state(&self, state: &RecordingSnapshot) {
            self.states
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(state.clone());
        }

        fn segment(&self, segment: &SegmentUpdate) {
            self.segments
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(segment.clone());
        }

        fn health(&self, notice: &HealthNotice) {
            self.health
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(notice.clone());
        }
    }
}
