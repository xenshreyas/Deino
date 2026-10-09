//! Legacy recording lifecycle commands (start, stop, status).
//!
//! Thin wrappers over `audio::recording_commands` that also refresh the tray
//! and show recording notifications. FND-05 replaces them.

use log::{error, info};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::audio::recording_commands as recorder;
use crate::notifications::commands::{
    show_recording_started_notification, show_recording_stopped_notification,
    NotificationManagerState,
};
use crate::tray;

/// Arguments of [`stop_recording`], sent by the UI as `{ args: { save_path } }`.
#[derive(Debug, Deserialize)]
pub struct RecordingArgs {
    save_path: String,
}

#[tauri::command]
pub async fn start_recording<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    info!(
        "start_recording called - mic: {:?}, system: {:?}, meeting: {:?}",
        mic_device_name, system_device_name, meeting_name
    );
    if is_recording().await {
        return Err("Recording already in progress".to_string());
    }

    recorder::start_recording_with_devices_and_meeting(
        app.clone(),
        mic_device_name,
        system_device_name,
        meeting_name.clone(),
    )
    .await
    .map_err(|e| {
        error!("Failed to start audio recording: {}", e);
        format!("Failed to start recording: {}", e)
    })?;

    tray::update_tray_menu(&app);
    info!("Recording started successfully");
    notify_recording_started(&app, meeting_name).await;
    Ok(())
}

#[tauri::command]
pub async fn stop_recording<R: Runtime>(app: AppHandle<R>, args: RecordingArgs) -> Result<(), String> {
    info!("Attempting to stop recording...");
    if !recorder::is_recording().await {
        info!("Recording is already stopped");
        return Ok(());
    }

    let stop_args = recorder::RecordingArgs { save_path: args.save_path.clone() };
    let result = recorder::stop_recording(app.clone(), stop_args).await;
    // Refresh the tray even when stopping failed, so it never shows a stale state.
    tray::update_tray_menu(&app);
    if let Err(e) = result {
        error!("Failed to stop audio recording: {}", e);
        return Err(format!("Failed to stop recording: {}", e));
    }

    ensure_parent_dir(&args.save_path)?;
    notify_recording_stopped(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn is_recording() -> bool {
    recorder::is_recording().await
}

/// Transcription queue status polled by the stop flow until nothing is left.
///
/// `chunks_in_queue` counts queued chunks (Whisper, Parakeet TDT) or seconds
/// of live audio not yet transcribed (Parakeet Unified); `is_processing` is
/// true while a transcription job runs; `last_activity_ms` is the time since
/// transcription last did anything.
#[derive(Debug, Serialize, Clone)]
pub struct TranscriptionStatus {
    chunks_in_queue: usize,
    is_processing: bool,
    last_activity_ms: u64,
}

#[tauri::command]
pub fn get_transcription_status<R: Runtime>(app: AppHandle<R>) -> TranscriptionStatus {
    let status = crate::stt::status(&app);
    TranscriptionStatus {
        chunks_in_queue: status.queued,
        is_processing: status.processing,
        last_activity_ms: status.idle_ms,
    }
}

#[tauri::command]
pub async fn start_recording_with_devices<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
) -> Result<(), String> {
    start_recording_with_devices_and_meeting(app, mic_device_name, system_device_name, None).await
}

#[tauri::command]
pub async fn start_recording_with_devices_and_meeting<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    info!(
        "start_recording_with_devices_and_meeting - mic: {:?}, system: {:?}, meeting: {:?}",
        mic_device_name, system_device_name, meeting_name
    );
    let result = if mic_device_name.is_none() && system_device_name.is_none() {
        recorder::start_recording_with_meeting_name(app.clone(), meeting_name.clone()).await
    } else {
        recorder::start_recording_with_devices_and_meeting(
            app.clone(),
            mic_device_name,
            system_device_name,
            meeting_name.clone(),
        )
        .await
    };

    if let Err(e) = result {
        error!("Failed to start recording via tauri command: {}", e);
        return Err(e);
    }
    info!("Recording started successfully via tauri command");
    notify_recording_started(&app, meeting_name).await;
    Ok(())
}

/// Creates the parent directory of `save_path` when it does not exist yet.
fn ensure_parent_dir(save_path: &str) -> Result<(), String> {
    let Some(parent) = std::path::Path::new(save_path).parent() else {
        return Ok(());
    };
    if parent.exists() {
        return Ok(());
    }
    info!("Creating directory: {:?}", parent);
    std::fs::create_dir_all(parent).map_err(|e| {
        let message = format!("Failed to create save directory: {}", e);
        error!("{}", message);
        message
    })
}

/// Shows the "recording started" notification, honouring user preferences.
async fn notify_recording_started<R: Runtime>(app: &AppHandle<R>, meeting_name: Option<String>) {
    let state = app.state::<NotificationManagerState<R>>();
    if let Err(e) = show_recording_started_notification(app, &state, meeting_name).await {
        error!("Failed to show recording started notification: {}", e);
    }
}

/// Shows the "recording stopped" notification, honouring user preferences.
async fn notify_recording_stopped<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<NotificationManagerState<R>>();
    if let Err(e) = show_recording_stopped_notification(app, &state).await {
        error!("Failed to show recording stopped notification: {}", e);
    }
}
