//! `network` feature: Settings → Privacy → Network (FND-12), exposed as the
//! inline plugin `plugin:network|<command>`.
//!
//! The page lists every outside service with its hosts and whether it may
//! send, switches services on and off, and shows the egress log: what left
//! the Mac, when, and how big it was, never what it carried. Offline mode is
//! a plain setting (`network.offlineMode`), changed through the `settings`
//! plugin. New egress log rows reach the UI through `DbChanged`
//! (`egress_log`), so this plugin declares no events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "network";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `network` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
