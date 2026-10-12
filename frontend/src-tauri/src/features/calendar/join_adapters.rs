//! The Join ports' adapters: [`SystemCallOpener`] opens call links with the
//! system (web links only, `system_open`), [`LiveRecorder`] starts the
//! live recorder, and [`RecorderPhase`] tells the meeting ticker whether it
//! is busy.
//!
//! The recorder is found in Tauri state at each Join, like
//! [`LinkOnStart`](super::LinkOnStart): it is built before or after the
//! calendar depending on startup order. The recording starts through the
//! same `RecordingService::start(StartRequest)` path as the menu bar's
//! Record (CAP-10), titled after the event and linked to it and its series.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::calendar::CalendarEvent;
use deino_core::calendar::join::{CallOpener, EventRecorder, PortError};
use deino_core::calendar::ticker::RecorderBusy;
use deino_core::meetings::MeetingLink;
use deino_domain::MeetingId;
use deino_recording::{RecordingPhase, RecordingService, StartRequest};
use tauri::{AppHandle, Manager, Runtime};

use crate::system_open::open_external_url;

/// Opens call links in the default browser or the meeting app.
pub struct SystemCallOpener;

#[async_trait]
impl CallOpener for SystemCallOpener {
    async fn open_call(&self, url: &str) -> Result<(), PortError> {
        Ok(open_external_url(url).await?)
    }
}

/// Starts the app's live recorder.
pub struct LiveRecorder<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> LiveRecorder<R> {
    /// A recorder over the `Arc<RecordingService>` that `app` manages, once it does.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

#[async_trait]
impl<R: Runtime> EventRecorder for LiveRecorder<R> {
    async fn record(&self, event: &CalendarEvent) -> Result<MeetingId, PortError> {
        let recorder = self
            .app
            .try_state::<Arc<RecordingService>>()
            .map(|state| Arc::clone(&state))
            .ok_or("the recorder is not ready yet")?;
        let request = StartRequest {
            title: Some(event.title.clone()).filter(|title| !title.trim().is_empty()),
            link: Some(MeetingLink {
                calendar_event_id: Some(event.id),
                series_id: event.series_id.clone(),
            }),
        };
        Ok(recorder.start(request).await?)
    }
}

/// Whether the live recorder is busy, for the meeting ticker.
pub struct RecorderPhase<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> RecorderPhase<R> {
    /// The phase of the `Arc<RecordingService>` that `app` manages, once it does.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> RecorderBusy for RecorderPhase<R> {
    fn is_recording(&self) -> bool {
        self.app
            .try_state::<Arc<RecordingService>>()
            .is_some_and(|recorder| recorder.state().phase != RecordingPhase::Idle)
    }
}
