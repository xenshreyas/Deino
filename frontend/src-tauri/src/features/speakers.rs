//! `speakers` feature: the IPC surface of `deino_core::speakers` and
//! `deino_core::voiceprints` (CAP-04), exposed as the inline plugin
//! `plugin:speakers|<command>`.
//!
//! - The meeting's participants panel: `participants`, `candidates`,
//!   `rename` (this segment or all segments), `merge`, and answering the
//!   `identify_speakers` stage's suggestions (`confirm_suggestion`,
//!   `dismiss_suggestion`), thin over `SpeakerService`;
//! - Settings → People → Voice profiles: `voice_profiles`,
//!   `delete_voiceprint` and `forget_voice`, thin over `VoiceprintService`.
//!
//! Both services are wired in `composition/speakers.rs`. Changes reach the UI
//! through `DbChanged` (`speakers`, `voiceprints`, `speaker_suggestions`,
//! `people`, `action_items`), so the plugin declares no events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "speakers";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `speakers` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
