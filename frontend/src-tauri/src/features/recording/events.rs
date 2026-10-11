//! The `recording` plugin's typed events. Names are namespaced by the
//! plugin: `plugin:recording:recording-state`, `plugin:recording:live-segment`,
//! `plugin:recording:health`, and for the record panel and auto-stop
//! (CAP-10, CAP-07) `audio-level`, `panel-pointer`, `panel-side`,
//! `auto-stop-changed` and `open-live-meeting`.

use deino_core::settings::PanelEdge;
use deino_detect::autostop::AutoStopState;
use deino_domain::MeetingId;
use deino_recording::{AudioLevels, HealthNotice, RecordingSnapshot, SegmentUpdate};
use serde::Serialize;
use tauri_specta::Event;

/// The recorder's state changed (phase, meeting, pause time, echo mode,
/// devices, health issues).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct RecordingState {
    /// The new state.
    pub snapshot: RecordingSnapshot,
}

/// A live transcript line grew, closed (saved) or was discarded as echo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct LiveSegment {
    /// The line, on its channel (Me = `mic`, Them = `system`).
    pub update: SegmentUpdate,
}

/// Something about the recording's audio the user may need to know.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    /// What happened.
    pub notice: HealthNotice,
}

/// How loud each track is, about 30 times a second, to the record panel only.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevel {
    /// The loudest sample per track since the last event.
    pub levels: AudioLevels,
}

/// The pointer entered or left the record panel's pill. The panel window
/// ignores the mouse outside the pill (clicks pass through to the app
/// below), so it cannot see hover itself; Rust tracks the pointer instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PanelPointer {
    /// The pointer is over the pill.
    pub inside: bool,
}

/// The record panel snapped to a screen edge; the pill hugs that side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PanelSide {
    /// The edge.
    pub edge: PanelEdge,
}

/// The "Ending…" notice or the "Start new note" offer changed (CAP-07).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct AutoStopChanged {
    /// What is shown now.
    pub state: AutoStopState,
}

/// The record panel asked for the live meeting's notes: the main window
/// opens the record screen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct OpenLiveMeeting {
    /// The meeting being recorded.
    pub meeting_id: MeetingId,
}
