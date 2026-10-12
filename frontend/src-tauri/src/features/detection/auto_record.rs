//! Auto-record on the microphone (ORG-02 with CAP-06): when a detected
//! call is linked to a calendar event whose rules record it, recording
//! starts at once, without the prompt or its countdown
//! (`StartTrigger::MicInUse`). Any other call is prompted as usual.

use std::sync::Arc;

use deino_core::calendar::rules::{AutoRecordService, StartTrigger};
use deino_detect::{Detection, DetectionService};
use tauri::{AppHandle, Manager, Wry};

/// True when `detection`'s event records on its own. Called on the
/// detector thread, outside the async runtime.
pub(super) fn records_on_its_own(app: &AppHandle<Wry>, detection: &Detection) -> bool {
    let Some(event) = &detection.event else {
        return false;
    };
    let Some(rules) = app
        .try_state::<Arc<AutoRecordService>>()
        .map(|rules| Arc::clone(&rules))
    else {
        return false;
    };
    let starts = rules.starts_on(event.id, StartTrigger::MicInUse);
    tauri::async_runtime::block_on(starts).unwrap_or_else(|error| {
        log::warn!(target: "detect", "auto-record rules for event {}: {error}", event.id);
        false
    })
}

/// Starts recording `detection` without the prompt.
pub(super) fn record_now(app: &AppHandle<Wry>, detection: &Detection) {
    let Some(service) = app.try_state::<Arc<DetectionService>>() else {
        return log::warn!(target: "detect", "detection is not wired; cannot auto-record");
    };
    if let Err(error) = service.record_now(detection.id) {
        log::warn!(target: "detect", "auto-record of prompt {} failed: {error}", detection.id.0);
    }
}
