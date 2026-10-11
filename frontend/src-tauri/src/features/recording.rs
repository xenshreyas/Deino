//! `recording` feature: the live recorder, exposed as the inline plugin
//! `plugin:recording|<command>` (FND-05b, CAP-02, CAP-07, CAP-10).
//!
//! - `commands.rs`: `start`, `stop`, `pause`, `resume`, `state`,
//!   `transcript`, `resume_meeting`, thin over
//!   `deino_recording::RecordingService` (the tray calls the same service);
//!   `commands/controls.rs`: the record panel's and auto-stop's commands;
//! - `events.rs`: the typed events; [`adapter`]'s [`TauriRecordingEvents`]
//!   emits the recorder's and tells its followers (the menu bar, the
//!   notifications in [`notify`], the [`panel`]);
//! - [`panel`], the floating record panel window (CAP-10);
//! - [`autostop`], auto-stop's ports over the recorder, the calendar, the
//!   settings and Tauri (CAP-07).
//!
//! Rust owns the recording: it creates the meeting, saves the transcript as
//! it is spoken and hands the meeting to the pipeline on stop. The UI only
//! calls these commands and listens.

pub mod adapter;
pub mod autostop;
mod commands;
#[cfg(test)]
mod contract_tests;
pub mod events;
mod notify;
pub mod panel;

use std::sync::Arc;

pub use adapter::{Follower, TauriRecordingEvents};
pub use commands::{MainWindow, Microphone, PanelRegion, TauriMainWindow};
pub use events::{
    AudioLevel, AutoStopChanged, Health, LiveSegment, OpenLiveMeeting, PanelPointer, PanelSide,
    RecordingState,
};
pub use panel::{PanelHost, RecordPanel};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

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
            Health,
            AudioLevel,
            PanelPointer,
            PanelSide,
            AutoStopChanged,
            OpenLiveMeeting
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
            app.manage(Arc::new(PanelHost::default()));
            let window: Arc<dyn MainWindow> = Arc::new(TauriMainWindow(app.clone()));
            app.manage(window);
            Ok(())
        })
        .build()
}
