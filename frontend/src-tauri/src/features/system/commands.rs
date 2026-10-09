//! Commands of the `system` feature plugin.

use tauri::{AppHandle, Runtime};

/// Returns the app version from `tauri.conf.json` (for example `"0.4.1"`).
///
/// Called from the UI as `invoke('plugin:system|app_version')`.
#[tauri::command]
pub(super) fn app_version<R: Runtime>(app: AppHandle<R>) -> String {
    app.package_info().version.to_string()
}
