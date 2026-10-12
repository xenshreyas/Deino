//! [`ScreenCapture`]: what the app holds. It starts a [`Capturer`] for a
//! recording's [`CapturePlan`], turns it off for one meeting, and reports
//! its [`CaptureStatus`] and the Screen Recording permission.

use std::sync::{Arc, Mutex, PoisonError};

use deino_domain::MeetingId;

use crate::capturer::{Capturer, Start};
use crate::error::ScreenError;
use crate::pick::{TargetChoice, choices};
use crate::plan::CapturePlan;
use crate::ports::{Platform, RecordingClock, SnapshotStore};
use crate::status::{CaptureStatus, StatusCell};

/// The log target.
const TARGET: &str = "deino_screen";

/// What [`ScreenCapture`] is built over.
#[derive(Clone)]
pub struct ScreenCaptureParts {
    /// This Mac's screen ports; `None` where there are none.
    pub platform: Option<Platform>,
    /// The recorder's clock.
    pub clock: Arc<dyn RecordingClock>,
    /// Where snapshot rows go.
    pub store: Arc<dyn SnapshotStore>,
}

/// The current capture.
struct Running {
    meeting: MeetingId,
    _capturer: Capturer,
}

/// Screen capture for every recording (see the module docs).
pub struct ScreenCapture {
    parts: ScreenCaptureParts,
    status: Arc<StatusCell>,
    current: Mutex<Option<Running>>,
}

impl ScreenCapture {
    /// Screen capture over `parts`; nothing is captured yet.
    pub fn new(parts: ScreenCaptureParts) -> Self {
        Self {
            parts,
            status: Arc::default(),
            current: Mutex::new(None),
        }
    }

    /// Whether this platform can capture the screen at all.
    pub const fn supported(&self) -> bool {
        self.parts.platform.is_some()
    }

    /// Whether Screen Recording is granted, read without prompting.
    pub fn permitted(&self) -> bool {
        self.parts
            .platform
            .as_ref()
            .is_some_and(|platform| platform.permission.granted())
    }

    /// Starts capturing for `plan`. A previous capture is stopped (it
    /// closes its video on its own thread). Without a platform, nothing
    /// happens.
    pub fn start(&self, plan: CapturePlan) {
        let Some(platform) = self.parts.platform.clone() else {
            return log::debug!(target: TARGET, "screen capture is not supported here");
        };
        let meeting = plan.meeting.clone();
        self.status.begin(&meeting, plan.video.is_some());
        let start = Start {
            plan,
            platform,
            clock: Arc::clone(&self.parts.clock),
            store: Arc::clone(&self.parts.store),
            status: Arc::clone(&self.status),
        };
        let running = match Capturer::spawn(start) {
            Ok(capturer) => Some(Running {
                meeting,
                _capturer: capturer,
            }),
            Err(error) => {
                log::warn!(target: TARGET, "could not start the screen capture thread: {error}");
                self.status.end(&meeting);
                None
            }
        };
        *self.lock() = running;
    }

    /// Stops capturing `meeting` (the user turned it off for this meeting).
    /// What was captured is kept. False when it was not being captured.
    pub fn turn_off(&self, meeting: &MeetingId) -> bool {
        let mut current = self.lock();
        if current
            .as_ref()
            .is_none_or(|running| &running.meeting != meeting)
        {
            return false;
        }
        // Dropping the handle stops the thread, which closes the video.
        *current = None;
        drop(current);
        self.status.end(meeting);
        true
    }

    /// The picker's entries: the displays and the other apps' windows.
    /// Fails without the permission rather than touching
    /// `ScreenCaptureKit`, which would make macOS ask.
    pub fn choices(&self) -> Result<Vec<TargetChoice>, ScreenError> {
        let platform = self.parts.platform.as_ref().ok_or(ScreenError::Unsupported)?;
        if !platform.permission.granted() {
            return Err(ScreenError::NotPermitted);
        }
        Ok(choices(
            &platform.windows.list()?,
            &platform.windows.displays()?,
        ))
    }

    /// What the capture is doing now.
    pub fn status(&self) -> CaptureStatus {
        self.status.get()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Running>> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
