//! The Join ports' adapters: [`SystemCallOpener`] opens call links with the
//! system (web links only, `system_open`), and [`LiveRecorder`] starts the
//! live recorder.
//!
//! The recorder is found in Tauri state at each Join, like [`LinkOnStart`]
//! (super::LinkOnStart): it is built before or after the calendar depending
//! on startup order. The recording is titled after the event, and
//! `LinkOnStart` links it to the event nearest to now, which is the one
//! being joined. TODO(CAP-10): pass the event in its `StartRequest` link.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::calendar::CalendarEvent;
use deino_core::calendar::join::{CallOpener, EventRecorder, PortError};
use deino_domain::MeetingId;
use deino_recording::RecordingService;
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
        let title = Some(event.title.clone()).filter(|title| !title.trim().is_empty());
        Ok(recorder.start(title).await?)
    }
}
