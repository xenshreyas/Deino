//! `ask` feature: the IPC surface of `deino_core::ask`, "Ask about this
//! meeting" (AI-02-lite, D13), exposed as the inline plugin
//! `plugin:ask|<command>`.
//!
//! - `commands.rs`: read and clear a meeting's thread, ask (a question or a
//!   recipe), and preview a recipe; thin over `AskService` (wired in
//!   `composition/ask.rs`);
//! - `events.rs`: [`AskStreamEvent`], an answer's pieces over the
//!   `tauri::ipc::Channel` the UI passes to each request.
//!
//! The thread is stored through the database writer, so `DbChanged` on
//! `ask_messages` keeps the UI fresh.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::AskStreamEvent;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "ask";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript
/// bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        // No events: `.events` drops the `Channel` placeholder type the
        // commands bring in, which would clash with the generated import.
        .events(tauri_specta::collect_events![])
        .typ::<AskStreamEvent>()
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `ask` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .build()
}
