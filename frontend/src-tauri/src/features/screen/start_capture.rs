//! [`CaptureOnStart`]: the recorder's [`StartObserver`] that starts the
//! screen capture of a recording, however it was started (the app, the
//! tray, a detected call), through the same start path as CAP-05's speaker
//! tags. Its stop is the recorder's: the capture follows the recorder's
//! playhead and closes its video when the recording ends.
//!
//! Each start gathers what `deino_screen::decide` needs: the screen
//! settings, the meeting app detection sees now, the user's choice for the
//! meeting, and the permission, read without prompting.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::screen::ScreenService;
use deino_detect::DetectionService;
use deino_detect::autostop::MicrophoneHolders as _;
use deino_domain::MeetingId;
use deino_recording::StartObserver;
use deino_screen::{CaptureSettings, FrameRate, PlanInput, ScreenCapture, decide};
use tauri::{AppHandle, Manager, Runtime};

use crate::paths::AppPaths;

/// Starts a screen capture for each recording that should have one.
pub struct CaptureOnStart<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> CaptureOnStart<R> {
    /// An observer over the `Arc<ScreenCapture>` that `app` manages.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    /// What `decide` looks at for `meeting`; `None` before the app is wired.
    async fn input(&self, meeting: &MeetingId, capture: &ScreenCapture) -> Option<PlanInput> {
        let screen = Arc::clone(&*self.app.try_state::<Arc<ScreenService>>()?);
        let settings = crate::features::settings::current(&self.app).await.screen;
        let recordings = AppPaths::resolve(&self.app).ok()?.recordings();
        let meeting_app = self
            .app
            .try_state::<Arc<DetectionService>>()
            .and_then(|detection| detection.meeting_app());
        Some(PlanInput {
            meeting: meeting.clone(),
            folder: deino_audio::recording_dir(&recordings, meeting),
            settings: CaptureSettings {
                detected_meetings: settings.detected_meetings,
                video: settings.video,
                frame_rate: FrameRate::new(settings.video_fps),
                snapshots: settings.snapshots,
            },
            meeting_app,
            turned_off: screen.turned_off(meeting).await,
            permitted: capture.permitted(),
        })
    }
}

#[async_trait]
impl<R: Runtime> StartObserver for CaptureOnStart<R> {
    async fn started(&self, meeting: &MeetingId, _titled: bool) -> Option<String> {
        let capture = Arc::clone(&*self.app.try_state::<Arc<ScreenCapture>>()?);
        let input = self.input(meeting, &capture).await?;
        match decide(input) {
            Ok(plan) => capture.start(plan),
            Err(skip) => log::debug!(target: "screen", "no screen capture for {meeting}: {skip:?}"),
        }
        None
    }
}
