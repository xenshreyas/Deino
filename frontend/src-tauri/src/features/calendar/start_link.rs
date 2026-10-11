//! [`LinkOnStart`]: the recorder's [`StartObserver`] that links a meeting
//! to the calendar event nearest to now as soon as it starts recording
//! (ORG-03), whether it was started from the app, the tray or a detected
//! call.
//!
//! The service is found in Tauri state at each start: the recorder is built
//! before or after the calendar depending on startup order, and records
//! without a link when calendars are not wired.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::calendar::linking::{EventLinkService, TitleRule};
use deino_domain::MeetingId;
use deino_recording::StartObserver;
use tauri::{AppHandle, Manager, Runtime};

/// Auto-links recordings to their calendar event.
pub struct LinkOnStart<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> LinkOnStart<R> {
    /// An observer over the `Arc<EventLinkService>` that `app` manages, once it does.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

#[async_trait]
impl<R: Runtime> StartObserver for LinkOnStart<R> {
    async fn started(&self, meeting: &MeetingId, titled: bool) -> Option<String> {
        let links = Arc::clone(&*self.app.try_state::<Arc<EventLinkService>>()?);
        let rule = if titled {
            TitleRule::Keep
        } else {
            TitleRule::FromEvent
        };
        match links.auto_link(meeting, rule).await {
            Ok(change) => change.and_then(|change| change.title),
            Err(error) => {
                log::warn!(target: "calendar", "could not link meeting {meeting} to its event: {error}");
                None
            }
        }
    }
}
