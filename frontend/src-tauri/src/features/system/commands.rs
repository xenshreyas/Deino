//! Commands of the `system` feature plugin.

use tauri::Runtime;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![app_version]
}

/// The app version, for example `"0.4.1"`. `tauri.conf.json` carries the
/// same version; `features/ipc_tests.rs` fails if the two drift apart.
#[tauri::command]
#[specta::specta]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
