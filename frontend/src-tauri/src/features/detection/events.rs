//! Typed events of the `detection` plugin, for example
//! `plugin:detection:meeting-detected`.

use deino_detect::{Detection, DetectionId};
use serde::Serialize;
use tauri_specta::Event;

/// A meeting was detected: the prompt shows it.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct MeetingDetected {
    /// The meeting.
    pub detection: Detection,
}

/// The detected meeting ended: its app let go of the microphone.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct MeetingEnded {
    /// The prompt it was shown in.
    pub id: DetectionId,
}

/// The prompt closed without the meeting ending (answered, timed out,
/// recording started, detection switched off).
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct PromptClosed {
    /// The prompt.
    pub id: DetectionId,
}

/// Start recording the detected meeting ("Take notes", or the auto-start
/// countdown ran out). The main window's shell starts it.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct RecordingRequested {
    /// The meeting.
    pub detection: Detection,
    /// The user asked for the notes ("Take notes"), so the main window comes
    /// forward; an auto-start records without taking focus from the call.
    pub open_notes: bool,
}
