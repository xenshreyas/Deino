//! Legacy audio-device commands: device listing, microphone permission and
//! input level metering for the device picker.

use log::info;
use tauri::{AppHandle, Runtime};

use crate::audio::{self, AudioDevice};

#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    audio::list_audio_devices()
        .await
        .map_err(|e| format!("Failed to list audio devices: {}", e))
}

#[tauri::command]
pub async fn trigger_microphone_permission() -> Result<bool, String> {
    audio::trigger_audio_permission()
        .map_err(|e| format!("Failed to trigger microphone permission: {}", e))
}

#[tauri::command]
pub async fn start_audio_level_monitoring<R: Runtime>(
    app: AppHandle<R>,
    device_names: Vec<String>,
) -> Result<(), String> {
    info!("Starting audio level monitoring for {} devices", device_names.len());
    audio::simple_level_monitor::start_monitoring(app, device_names)
        .await
        .map_err(|e| format!("Failed to start audio level monitoring: {}", e))
}

#[tauri::command]
pub async fn stop_audio_level_monitoring() -> Result<(), String> {
    info!("Stopping audio level monitoring");
    audio::simple_level_monitor::stop_monitoring()
        .await
        .map_err(|e| format!("Failed to stop audio level monitoring: {}", e))
}

#[tauri::command]
pub async fn is_audio_level_monitoring() -> bool {
    audio::simple_level_monitor::is_monitoring()
}
