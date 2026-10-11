//! Wiring for meeting detection (CAP-06): the service over the app's
//! adapters, and the detector thread over this Mac's sensors.
//!
//! Calendar association waits for ORG-01, so the detector links no events
//! yet ([`NoCalendar`]); recording starts on the live recorder
//! (`features::detection::recorder`).

use std::sync::Arc;

use deino_core::settings::SettingsService;
use deino_detect::{
    DetectionPorts, DetectionService, Detector, DetectorParts, NoCalendar, platform_sensors,
};
use deino_domain::{Clock, SystemClock};
use tauri::{AppHandle, Manager};

use crate::features::detection::adapters::{SettingsDetection, TauriDetectionSink};
use crate::features::detection::recorder::{RecorderStarter, RecorderStatus};

/// Manages `Arc<DetectionService>` and starts the detector, which lives in
/// Tauri state until the app exits. Needs `attach_settings` to have run.
pub(crate) fn attach_detection(app: &AppHandle) {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        log::error!(target: "detect", "settings were not wired before detection; it is off");
        return;
    };
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let service = Arc::new(service(app, Arc::clone(&settings), Arc::clone(&clock)));
    app.manage(Arc::clone(&service));
    let parts = DetectorParts {
        service,
        calendar: Arc::new(NoCalendar),
        clock,
    };
    match Detector::spawn(platform_sensors, parts) {
        Ok(detector) => {
            app.manage(detector);
        }
        Err(error) => log::error!(target: "detect", "meeting detection did not start: {error}"),
    }
}

fn service(
    app: &AppHandle,
    settings: Arc<SettingsService>,
    clock: Arc<dyn Clock>,
) -> DetectionService {
    let settings = Arc::new(SettingsDetection::new(settings));
    let ports = DetectionPorts {
        sink: Arc::new(TauriDetectionSink::new(app.clone())),
        recorder: Arc::new(RecorderStarter::new(app.clone())),
        status: Arc::new(RecorderStatus::new(app.clone())),
        config: Arc::clone(&settings) as _,
        ignore_list: settings,
    };
    DetectionService::new(ports, clock)
}
