//! `integrations` feature: Settings → Integrations (INT-00, ADR 0029),
//! exposed as the inline plugin `plugin:integrations|<command>`.
//!
//! The commands are thin over `deino_integrations::IntegrationService`:
//! the providers with their ways to connect and their connections; connect
//! in the browser (and cancel), paste a token, check, disconnect; the user's
//! own OAuth app; where a connection can write; delivering a write; and the
//! delivery log. Tokens only ever travel inward: a pasted token and a client
//! secret go to the Keychain, and nothing returns a secret. Connections and
//! deliveries reach the UI through `DbChanged`
//! (`integration_connections`, `integration_connection_scopes`,
//! `integration_deliveries`), so this plugin declares no events.
//!
//! The service, its Keychain vault, the network egress and the browser are
//! wired in `composition/integrations.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "integrations";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `integrations` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
