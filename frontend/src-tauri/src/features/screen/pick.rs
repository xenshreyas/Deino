//! [`ScreenPicker`]: the record panel's window or screen picker. A pick
//! captures what the user chose for the current recording, whether or not
//! a meeting app was detected (so a manual start can capture too), and
//! turns capture back on for the meeting. A recording that already has a
//! video keeps it: the new capture keeps snapshots only (`deino_screen::picked`).

use std::sync::Arc;

use deino_core::screen::ScreenService;
use deino_core::settings::SettingsService;
use deino_domain::{AppError, MeetingId};
use deino_screen::{PickInput, ScreenCapture, ScreenError, Skip, Source, has_video, picked};

use super::start_capture::capture_settings;

/// The picker over the capture, the off switch and the settings.
pub struct ScreenPicker {
    capture: Arc<ScreenCapture>,
    screen: Arc<ScreenService>,
    settings: Arc<SettingsService>,
}

impl ScreenPicker {
    /// A picker over the app's screen capture and settings.
    pub fn new(
        capture: Arc<ScreenCapture>,
        screen: Arc<ScreenService>,
        settings: Arc<SettingsService>,
    ) -> Self {
        Self {
            capture,
            screen,
            settings,
        }
    }

    /// Captures `source` for `meeting` from now on, replacing a running
    /// capture. False when Settings keeps neither video nor snapshots.
    pub async fn pick(&self, meeting: &MeetingId, source: Source) -> Result<bool, AppError> {
        let folder = self.screen.recording_folder(meeting);
        let input = PickInput {
            meeting: meeting.clone(),
            has_video: has_video(&folder),
            folder,
            settings: capture_settings(self.settings.get().await.screen),
            source,
            permitted: self.capture.permitted(),
        };
        let plan = match picked(input) {
            Ok(plan) => plan,
            Err(Skip::NoPermission) => return Err(ScreenError::NotPermitted.into()),
            Err(_) => return Ok(false),
        };
        self.screen.turn_on(meeting).await.map_err(AppError::from)?;
        self.capture.start(plan);
        Ok(true)
    }
}
