//! Recording folders and the macOS system-audio backend.
//!
//! Recording preferences (auto-save, preferred devices, the recording
//! notice) moved to the typed settings in FND-06; this module keeps the
//! recordings folder and the backend commands.

use log::{info, warn};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

use anyhow::Result;
#[cfg(target_os = "macos")]
use log::error;

/// Name of the folder that holds recordings inside the user's media folder.
const RECORDINGS_FOLDER_NAME: &str = "Deino";

/// Get the default recordings folder based on platform
///
/// - macOS: `~/Movies/Deino`
/// - Windows: `%USERPROFILE%\Music\Deino`
/// - Linux and others: `~/Documents/Deino`
///
/// Falls back to the Documents folder when the media folder is unavailable.
pub fn get_default_recordings_folder() -> PathBuf {
    #[cfg(target_os = "windows")]
    let media_dir = dirs::audio_dir();
    #[cfg(target_os = "macos")]
    let media_dir = dirs::video_dir();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let media_dir: Option<PathBuf> = None;

    media_dir
        .or_else(dirs::document_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(RECORDINGS_FOLDER_NAME)
}

/// Ensure the recordings directory exists
pub fn ensure_recordings_directory(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
        info!("Created recordings directory: {:?}", path);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_default_recordings_folder_path() -> Result<String, String> {
    let path = get_default_recordings_folder();
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn open_recordings_folder<R: Runtime>(_app: AppHandle<R>) -> Result<(), String> {
    let save_folder = get_default_recordings_folder();

    // Ensure directory exists before trying to open it
    ensure_recordings_directory(&save_folder)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let folder_path = save_folder.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    info!("Opened recordings folder: {}", folder_path);
    Ok(())
}

#[tauri::command]
pub async fn select_recording_folder<R: Runtime>(
    _app: AppHandle<R>,
) -> Result<Option<String>, String> {
    // Use Tauri's dialog to select folder
    // For now, return None - this would need to be implemented with tauri-plugin-dialog
    // when it's available in the Cargo.toml
    warn!("Folder selection not yet implemented - using dialog plugin");
    Ok(None)
}
