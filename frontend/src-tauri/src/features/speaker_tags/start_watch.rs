//! [`WatchOnStart`]: the recorder's [`StartObserver`] that starts reading
//! the meeting app's speaker names as soon as a recording starts, however
//! it was started (the app, the tray, a detected call).
//!
//! Speaker tags are found in Tauri state at each start; without them the
//! meeting is named by voice alone.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::MeetingId;
use deino_recording::StartObserver;
use deino_speaker_tags::SpeakerTags;
use tauri::{AppHandle, Manager, Runtime};

/// Starts a speaker-tag watch for each recording.
pub struct WatchOnStart<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> WatchOnStart<R> {
    /// An observer over the `Arc<SpeakerTags>` that `app` manages.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

#[async_trait]
impl<R: Runtime> StartObserver for WatchOnStart<R> {
    async fn started(&self, meeting: &MeetingId, _titled: bool) -> Option<String> {
        let tags = self.app.try_state::<Arc<SpeakerTags>>()?;
        tags.watch(meeting.clone());
        None
    }
}
