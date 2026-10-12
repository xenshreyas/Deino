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
//! A feature registers itself from these files alone: `register_plugin!`
//! below adds the plugin (and its bindings) to the app, `build.rs` reads
//! `commands.rs`'s `collect_commands!` for its ACL and capability, and
//! `composition/example.rs` wires its service with an attach hook. The only
//! shared lines are its `pub mod` in `features.rs` and `mod` in
//! `composition.rs`. The example is compiled into debug builds only, which
//! the `cfg` below declares (and `build.rs` reads).

#![cfg(debug_assertions)]

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{NoteAdded, TauriExampleEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "example";

super::registry::register_plugin!(Guarded);

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
