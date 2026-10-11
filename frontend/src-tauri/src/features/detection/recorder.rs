//! Detection's recording ports over the live recorder (CAP-02).
//!
//! - [`RecorderStarter`]: starts `deino_recording::RecordingService` for a
//!   detected meeting, titled after its calendar event or its app, and
//!   tells the main window ([`RecordingRequested`]) so "Take notes" opens
//!   the record screen;
//! - [`RecorderStatus`]: whether the recorder is busy.
//!
//! The recorder is looked up on each call: detection starts before the
//! database (and with it the recorder) is ready.

use std::sync::Arc;

use deino_detect::{
    DetectError, Detection, MeetingKind, RecordingRequest, RecordingStarter, RecordingStatus,
};
use deino_recording::{RecordingPhase, RecordingService};
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use super::events::RecordingRequested;

/// The live recorder, once it is managed.
fn recorder<R: Runtime>(app: &AppHandle<R>) -> Option<Arc<RecordingService>> {
    app.try_state::<Arc<RecordingService>>()
        .map(|state| Arc::clone(&state))
}

/// The title of a detected meeting's recording: its calendar event's, or
/// the app and kind ("Zoom meeting", "Slack huddle").
pub fn meeting_title(detection: &Detection) -> String {
    if let Some(event) = detection
        .event
        .as_ref()
        .filter(|event| !event.title.trim().is_empty())
    {
        return event.title.clone();
    }
    let kind = match detection.kind {
        MeetingKind::Meeting => "meeting",
        MeetingKind::Huddle => "huddle",
        MeetingKind::Call => "call",
    };
    format!("{} {kind}", detection.app.name)
}

/// Starts the recorder for detected meetings.
pub struct RecorderStarter<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> RecorderStarter<R> {
    /// A starter over `app`'s recorder.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> RecordingStarter for RecorderStarter<R> {
    fn start(&self, request: &RecordingRequest) -> Result<(), DetectError> {
        let recorder = recorder(&self.app)
            .ok_or_else(|| DetectError::Recording("the recorder is not ready yet".into()))?;
        if request.open_notes {
            crate::tray::focus_main_window(&self.app);
        }
        let requested = RecordingRequested {
            detection: request.detection.clone(),
            open_notes: request.open_notes,
        };
        if let Err(error) = requested.emit(&self.app) {
            log::warn!(target: "detect", "could not tell the window about the recording: {error}");
        }
        let (app, title) = (self.app.clone(), meeting_title(&request.detection));
        tauri::async_runtime::spawn(async move {
            if !crate::features::settings::current(&app)
                .await
                .onboarding
                .completed
            {
                return log::info!(target: "detect", "not recording a detected meeting before setup is finished");
            }
            if let Err(error) = recorder.start(Some(title)).await {
                log::warn!(target: "detect", "the detected meeting did not start recording: {error}");
            }
        });
        Ok(())
    }
}

/// Whether the live recorder is recording.
pub struct RecorderStatus<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> RecorderStatus<R> {
    /// The status of `app`'s recorder.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> RecordingStatus for RecorderStatus<R> {
    fn is_recording(&self) -> bool {
        recorder(&self.app).is_some_and(|recorder| recorder.state().phase != RecordingPhase::Idle)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use deino_detect::{AppIdentity, DetectionId, LinkedEvent, Platform};
    use deino_domain::{BundleId, CalendarEventId};

    use super::*;

    fn zoom(event: Option<&str>) -> Detection {
        let at = Utc.with_ymd_and_hms(2026, 10, 9, 10, 0, 0).unwrap();
        Detection {
            id: DetectionId(1),
            app: AppIdentity {
                bundle_id: BundleId::try_from("us.zoom.xos").unwrap(),
                name: "Zoom".into(),
            },
            kind: MeetingKind::Meeting,
            platform: Platform::Zoom,
            event: event.map(|title| LinkedEvent {
                id: CalendarEventId::generate(&deino_domain::SystemClock),
                title: title.into(),
                starts_at: at,
            }),
            detected_at: at,
            auto_start_at: None,
        }
    }

    #[test]
    fn a_detected_meeting_is_titled_after_its_event_or_its_app() {
        assert_eq!(meeting_title(&zoom(Some("Weekly sync"))), "Weekly sync");
        assert_eq!(meeting_title(&zoom(Some("  "))), "Zoom meeting");
        assert_eq!(meeting_title(&zoom(None)), "Zoom meeting");
    }
}
