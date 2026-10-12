//! `import` feature (CAP-11): the IPC surface of `deino_core::import` and
//! `deino_core::watch`, exposed as the inline plugin `plugin:import|<command>`.
//!
//! - Importing a recording: `pick_file` (the open dialog), `check` (a
//!   dropped file) and `start`, which returns the new meeting once its audio
//!   is converted and it is `processing`;
//! - Settings → Watched folders: `watched_folders`, `add_watched_folder`
//!   (the folder dialog), `remove_watched_folder`, `set_move_to_processed`,
//!   and the failure list (`watch_failures`, `retry_failure`,
//!   `dismiss_failure`).
//!
//! Dialogs open in Rust (`platform.rs`); `check` and `start` take a path
//! only because a file dropped on the window arrives as one, and both
//! accept nothing but an existing media file. The services and the folder
//! scheduler are wired in `composition/import.rs`; changes reach the UI
//! through `DbChanged` (`meetings`, `watched_folders`, `watched_files`).

mod commands;
#[cfg(test)]
mod contract_tests;
mod platform;

pub use platform::TauriImportDialogs;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "import";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `import` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
