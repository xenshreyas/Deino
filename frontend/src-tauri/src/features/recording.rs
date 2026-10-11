//! `recording` feature: the live recorder, exposed as the inline plugin
//! `plugin:recording|<command>` (FND-05b, CAP-02).
//!
//! - `commands.rs`: `start`, `stop`, `pause`, `resume`, `state`, thin over
//!   `deino_recording::RecordingService` (the tray calls the same service);
//! - `events.rs`: the typed `RecordingState`, `LiveSegment` and `Health`
//!   events, emitted by [`TauriRecordingEvents`], which also refreshes the
//!   tray and shows the started/stopped notifications.
//!
//! Rust owns the recording: it creates the meeting, saves the transcript as
//! it is spoken and hands the meeting to the pipeline on stop. The UI only
//! calls these commands and listens.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{Health, LiveSegment, RecordingState, TauriRecordingEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "recording";

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![
            RecordingState,
            LiveSegment,
            Health
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `recording` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app, _| {
            builder.mount_events(app);
            Ok(())
        })
        .build()
}
