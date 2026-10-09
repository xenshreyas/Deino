//! `example` feature: the IPC half of the reference feature
//! (`deino_core::_example`). Copy this module to start a new feature plugin.
//!
//! A feature plugin has three parts:
//!
//! - [`specta`]: the one `tauri_specta::Builder` listing the commands and
//!   events. The invoke handler and the TypeScript bindings both come from it,
//!   so they cannot drift.
//! - `commands.rs`: thin commands. Each takes its service from Tauri state,
//!   calls one method and maps the error with `into_ipc()`.
//! - `events.rs`: typed events, plus the adapter that implements the core's
//!   event port by emitting them.
//!
//! Registering a feature takes one line in each of `features/manifest.rs`,
//! `lib.rs` (`.plugin(..)`), `capabilities/default.json` and
//! `features::builders()`, plus its services in `composition.rs`. The example
//! is compiled into debug builds only, so its manifest entry is in
//! `DEV_FEATURES` and its capability in `capabilities/dev/`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{NoteAdded, TauriExampleEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "example";

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![NoteAdded])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `example` plugin.
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
