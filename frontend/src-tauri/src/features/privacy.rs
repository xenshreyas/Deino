//! `privacy` feature (PRV-02): the app lock and the library's startup
//! status, exposed as the inline plugin `plugin:privacy|<command>`.
//!
//! - [`commands`]: `state`, `touch` (activity), `lock`, `unlock` over
//!   `deino_core::lock::LockService`, and `library` (the startup status);
//! - [`guard`]: the IPC guard every feature plugin is registered behind,
//!   which refuses library reads while the app is locked;
//! - [`library`]: where opening the (possibly encrypted) database stands.
//!
//! The service, the wake observer and the `LocalAuthentication` adapter are
//! wired in `composition/privacy.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;
pub(crate) mod guard;
pub(crate) mod library;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "privacy";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `privacy` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
