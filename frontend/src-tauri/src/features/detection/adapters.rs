//! The app's implementations of `deino-detect`'s output ports.
//!
//! - [`TauriDetectionSink`]: emits the plugin's events and shows or hides
//!   the prompt window.
//! - [`ShellRecordingStarter`]: asks the main window's shell to start
//!   recording, the same path as the sidebar, ⌘⇧R and the tray, because the
//!   record screen still owns the recording lifecycle (CAP-02 replaces this
//!   with its typed `recording.start`).
//! - [`LegacyRecordingStatus`]: the legacy recorder's flag.
//! - [`SettingsDetection`]: the `detection` section of the typed settings.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_core::settings::{IgnoredApp, SettingsService};
use deino_detect::{
    AppIdentity, ConfigSource, DetectError, DetectionConfig, DetectionEvent, DetectionSink,
    IgnoreList, RecordingRequest, RecordingStarter, RecordingStatus,
};
use tauri::{AppHandle, Wry};
use tauri_specta::Event;

use super::events::{MeetingDetected, MeetingEnded, PromptClosed, RecordingRequested};
use super::prompt;

/// Publishes detection events to the UI and drives the prompt window.
pub struct TauriDetectionSink {
    app: AppHandle<Wry>,
}

impl TauriDetectionSink {
    /// A sink that emits through `app`.
    pub fn new(app: AppHandle<Wry>) -> Self {
        Self { app }
    }
}

impl DetectionSink for TauriDetectionSink {
    fn publish(&self, event: DetectionEvent) {
        log::info!(target: "detect", "prompt {}: {}", event.id().0, label(&event));
        let emitted = match event {
            DetectionEvent::MeetingDetected(detection) => {
                let emitted = MeetingDetected { detection }.emit(&self.app);
                prompt::show(&self.app);
                emitted
            }
            DetectionEvent::MeetingEnded(id) => {
                prompt::hide(&self.app);
                MeetingEnded { id }.emit(&self.app)
            }
            DetectionEvent::PromptClosed(id) => {
                prompt::hide(&self.app);
                PromptClosed { id }.emit(&self.app)
            }
        };
        if let Err(error) = emitted {
            log::warn!(target: "detect", "could not emit a detection event: {error}");
        }
    }
}

/// What happened, for the log (never the app's window titles or URLs).
fn label(event: &DetectionEvent) -> &'static str {
    match event {
        DetectionEvent::MeetingDetected(_) => "meeting detected",
        DetectionEvent::MeetingEnded(_) => "meeting ended",
        DetectionEvent::PromptClosed(_) => "closed",
    }
}

/// Starts recording through the main window's shell.
pub struct ShellRecordingStarter {
    app: AppHandle<Wry>,
}

impl ShellRecordingStarter {
    /// A starter that asks `app`'s main window.
    pub fn new(app: AppHandle<Wry>) -> Self {
        Self { app }
    }
}

impl RecordingStarter for ShellRecordingStarter {
    fn start(&self, request: &RecordingRequest) -> Result<(), DetectError> {
        if request.open_notes {
            crate::tray::focus_main_window(&self.app);
        }
        let event = RecordingRequested {
            detection: request.detection.clone(),
            open_notes: request.open_notes,
        };
        event
            .emit(&self.app)
            .map_err(|error| DetectError::Recording(Box::new(error)))
    }
}

/// Whether the legacy recorder is recording.
pub struct LegacyRecordingStatus;

impl RecordingStatus for LegacyRecordingStatus {
    fn is_recording(&self) -> bool {
        // An atomic read behind an `async fn`; never waits.
        tauri::async_runtime::block_on(crate::audio::recording_commands::is_recording())
    }
}

/// Detection's view of the typed settings.
pub struct SettingsDetection {
    settings: Arc<SettingsService>,
}

impl SettingsDetection {
    /// Reads and writes `settings`.
    pub fn new(settings: Arc<SettingsService>) -> Self {
        Self { settings }
    }
}

impl ConfigSource for SettingsDetection {
    fn config(&self) -> DetectionConfig {
        // Called on the detector thread, outside the async runtime.
        let detection = tauri::async_runtime::block_on(self.settings.get()).detection;
        let countdown = TimeDelta::seconds(i64::from(detection.auto_start.countdown_seconds));
        DetectionConfig {
            enabled: detection.enabled,
            ignored: detection
                .ignored_apps
                .into_iter()
                .map(|app| app.bundle_id)
                .collect(),
            auto_start: detection.auto_start.enabled.then_some(countdown),
        }
    }
}

#[async_trait]
impl IgnoreList for SettingsDetection {
    async fn ignore(&self, app: &AppIdentity) -> Result<(), DetectError> {
        let ignored = IgnoredApp {
            bundle_id: app.bundle_id.clone(),
            name: app.name.clone(),
        };
        self.settings
            .update(move |settings| settings.detection.ignore(ignored))
            .await
            .map(drop)
            .map_err(|error| DetectError::IgnoreList(Box::new(error)))
    }
}
