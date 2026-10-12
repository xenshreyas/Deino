//! The app's implementations of `deino-detect`'s output ports.
//!
//! - [`TauriDetectionSink`]: emits the plugin's events and shows or hides
//!   the prompt window; a call whose event auto-records skips the prompt
//!   and records (`auto_record`, ORG-02).
//! - the recording ports are in [`super::recorder`] (the live recorder).
//! - [`SettingsDetection`]: the `detection` section of the typed settings.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_core::settings::{IgnoredApp, SettingsService};
use deino_detect::{
    AppIdentity, ConfigSource, DetectError, DetectionConfig, DetectionEvent, DetectionSink,
    IgnoreList,
};
use tauri::{AppHandle, Wry};
use tauri_specta::Event;

use super::events::{MeetingDetected, MeetingEnded, PromptClosed};
use super::{auto_record, prompt};

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
                let emitted = MeetingDetected {
                    detection: detection.clone(),
                }
                .emit(&self.app);
                if auto_record::records_on_its_own(&self.app, &detection) {
                    auto_record::record_now(&self.app, &detection);
                } else {
                    prompt::show(&self.app);
                }
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
