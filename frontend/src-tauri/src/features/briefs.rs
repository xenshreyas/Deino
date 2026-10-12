//! `briefs` feature: the IPC surface of `deino_core::briefs` (NOT-08),
//! exposed as the inline plugin `plugin:briefs|<command>`.
//!
//! - `commands.rs`: read an event's or meeting note's brief, mark it read,
//!   give feedback; thin over `BriefService` (wired in
//!   `composition/briefs.rs`);
//! - `job.rs`: ticks the background `BriefScheduler` once a minute.
//!
//! Brief writes go through the database writer, so `DbChanged` on `briefs`
//! keeps the UI fresh; there are no feature events.

mod commands;
#[cfg(test)]
mod contract_tests;
mod job;

pub(crate) use job::start_brief_job;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "briefs";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript
/// bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `briefs` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .build()
}
