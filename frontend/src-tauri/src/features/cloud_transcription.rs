//! `cloud-transcription` feature (CAP-14): ElevenLabs transcription after
//! the meeting on the user's own key, exposed as the inline plugin
//! `plugin:cloud-transcription|<command>`.
//!
//! - The key: `key_status`, `save_key` (switches `ServiceId::ElevenLabs`
//!   on), `check_key` (asks ElevenLabs) and `forget_key`. The UI never reads
//!   a key back.
//! - A meeting: `retranscribe` sends it to ElevenLabs and runs its
//!   transcript step and every later step again; `has_original` and
//!   `restore_original` bring back the transcript made on this Mac.
//!
//! The switch itself ("Transcribe meetings with ElevenLabs after they end")
//! and the model are settings (`transcription.cloudProvider`,
//! `transcription.cloudModel`). The plugin is guarded: nothing here works
//! while Deino is locked. Services are wired in
//! `composition/cloud_transcription.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "cloud-transcription";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `cloud-transcription` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
