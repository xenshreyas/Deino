// The `recording` plugin's commands, included by `manifest.rs` (so it holds
// plain items only: no `//!` docs, no `use`).

/// Every command of the `recording` plugin (features/recording.rs).
pub const RECORDING_COMMANDS: &[&str] = &[
    "start",
    "stop",
    "pause",
    "resume",
    "state",
    "resume_meeting",
    "set_microphone",
    "set_system_audio",
    "microphones",
    "keep_recording",
    "answer_split",
    "auto_stop",
    "open_live_meeting",
    "set_panel_region",
];
