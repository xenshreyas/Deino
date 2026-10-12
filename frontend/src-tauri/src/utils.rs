pub fn format_timestamp(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}

/// Opens a Privacy & Security pane of macOS System Settings, such as
/// `Privacy_Microphone`. Only plain pane names are accepted (`system_open`).
#[tauri::command]
pub async fn open_system_settings(preference_pane: String) -> Result<(), String> {
    let url = crate::system_open::settings_pane_url(&preference_pane).map_err(|e| e.to_string())?;
    crate::system_open::launch(std::ffi::OsStr::new(&url))
        .await
        .map_err(|e| format!("Failed to open system settings: {}", e))
} 