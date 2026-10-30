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
    /// "I use headphones": the microphone never hears the call, so echo
    /// cancellation and the bleed holdback are skipped whatever the output
    /// route looks like (CAP-02).
    pub uses_headphones: bool,
}

/// The recording section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct RecordingSettings {
    /// Keep the meeting's audio file. Off records the transcript only.
    pub auto_save: bool,
    /// Remind the user to tell participants that the meeting is recorded.
    pub show_recording_notice: bool,
    /// Show the floating record panel while recording (CAP-10). Off by
    /// default: the menu bar icon's red dot says a recording is on.
    pub show_panel: bool,
    /// Where the record panel was last left; `None` puts it at the right
    /// edge, a third of the way down.
    pub panel_position: Option<PanelPosition>,
    /// When a recording ends on its own (CAP-07).
    pub auto_stop: AutoStopOptions,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            auto_save: true,
            show_recording_notice: true,
            show_panel: false,
            panel_position: None,
            auto_stop: AutoStopOptions::default(),
        }
    }
}

/// The screen edge the record panel is snapped to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PanelEdge {
    /// The left edge.
    Left,
    /// The right edge (Granola's nub).
    #[default]
    Right,
}

/// Where the record panel sits: an edge, and how far down it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PanelPosition {
    /// The edge it hugs.
    pub edge: PanelEdge,
    /// How far down the screen's work area, in thousandths (0 = top).
    pub from_top: u16,
}

/// When a recording ends on its own, and the back-to-back prompt (CAP-07).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AutoStopOptions {
    /// End the recording when the call app lets go of the microphone, after
    /// 15 minutes without speech, or when the Mac sleeps (each with a
    /// "Keep recording" notice, sleep aside).
    pub enabled: bool,
    /// Also end it 10 minutes after the calendar event's end once it went
    /// quiet. Off by default: meetings often run over.
    pub after_calendar_end: bool,
    /// Offer a new note when the next calendar meeting begins with other
    /// people while recording.
    pub offer_next_meeting: bool,
}

impl Default for AutoStopOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            after_calendar_end: false,
            offer_next_meeting: true,
        }
    }
}

crate::settings::register_section! {
    /// Capture devices.
    audio: AudioSettings,
    order: 20,
}

crate::settings::register_section! {
    /// Recording behaviour.
    recording: RecordingSettings,
    order: 30,
}
