//! `webhooks` feature (DEV-01): the IPC surface of
//! `deino_core::webhooks`, exposed as the inline plugin
//! `plugin:webhooks|<command>` for Settings → Developer → Webhooks.
//!
//! - `commands.rs`: endpoints (list, create with the secret shown once,
//!   update, delete, rotate the secret), "Send test", and the delivery log;
//!   thin over `WebhookService`.
//!
//! The service, the event-port taps and the dispatch loop are wired in
//! `composition/webhooks.rs`. There are no events: the page refreshes
//! through `DbChanged` (`webhooks`, `webhook_deliveries`).

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "webhooks";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `webhooks` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
