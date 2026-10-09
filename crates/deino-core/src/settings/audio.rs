//! The audio and recording sections: which devices to capture and how a
//! recording behaves.

use serde::{Deserialize, Serialize};

/// The audio section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioSettings {
    /// The microphone to record, by its display name; the system default when `None`.
    pub preferred_mic_device: Option<String>,
    /// The system-audio source, by its display name; the default when `None`.
    pub preferred_system_device: Option<String>,
}

/// The recording section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct RecordingSettings {
    /// Keep the meeting's audio file. Off records the transcript only.
    pub auto_save: bool,
    /// Remind the user to tell participants that the meeting is recorded.
    pub show_recording_notice: bool,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            auto_save: true,
            show_recording_notice: true,
        }
    }
}
