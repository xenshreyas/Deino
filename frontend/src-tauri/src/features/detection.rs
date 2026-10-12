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
//! - [`adapters`]: the output ports (events and window, settings);
//!   [`recorder`]: recording start and status over the live recorder;
//! - `auto_record` (ORG-02): a call whose calendar event the user's rules
//!   record starts recording at once instead of prompting.
//!
//! The composition root starts the detector (`composition::attach_detection`).

pub mod adapters;
mod auto_record;
mod commands;
#[cfg(test)]
mod contract_tests;
mod events;
pub mod prompt;
pub mod recorder;

pub use commands::DetectionOverview;
pub use events::{MeetingDetected, MeetingEnded, PromptClosed, RecordingRequested};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "detection";

super::registry::register_plugin!(Open);

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
