//! `detection` feature (CAP-06): meeting detection and its prompt, exposed as
//! the inline plugin `plugin:detection|<command>`.
//!
//! `deino-detect` watches the Mac and decides; this plugin is its IPC and
//! its Tauri adapters:
//!
//! - commands answer the prompt's buttons (`take_notes`, `dismiss`,
//!   `ignore_app`), give a newly opened prompt the current detection
//!   (`current`) and report what this Mac supports (`settings`);
//! - events: [`MeetingDetected`], [`MeetingEnded`], [`PromptClosed`], and
//!   [`RecordingRequested`] for the main window's shell;
//! - [`prompt`]: the always-on-top, non-activating prompt window;
//! - [`adapters`]: the output ports (events and window, recording start,
//!   recording status, settings).
//!
//! The composition root starts the detector (`composition::attach_detection`).

pub mod adapters;
mod commands;
#[cfg(test)]
mod contract_tests;
mod events;
pub mod prompt;

pub use commands::DetectionOverview;
pub use events::{MeetingDetected, MeetingEnded, PromptClosed, RecordingRequested};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "detection";

/// The plugin's commands and events, for the invoke handler and the bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![
            MeetingDetected,
            MeetingEnded,
            PromptClosed,
            RecordingRequested
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `detection` plugin.
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
