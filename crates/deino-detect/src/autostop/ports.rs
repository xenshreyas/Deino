//! What auto-stop reads and acts through. The app implements the recording,
//! calendar, settings and action ports; [`crate::DetectionService`]
//! implements [`MicrophoneHolders`].

use chrono::{DateTime, Utc};
use deino_domain::{BundleId, CalendarEventId, MeetingId, Millis};

use super::boundary::EventWindow;
use super::policy::StopReason;
use super::service::AutoStopState;

/// Speech each lane has heard so far (totals only grow).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpeechTotals {
    /// Speech on the microphone so far.
    pub mic: Millis,
    /// Speech on the call (system audio) so far.
    pub system: Millis,
    /// Someone speaks on the microphone right now.
    pub mic_speaking: bool,
    /// Someone speaks on the call right now.
    pub system_speaking: bool,
}

/// The running recording, as auto-stop needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingView {
    /// The meeting being recorded.
    pub meeting: MeetingId,
    /// When its recording started.
    pub started_at: DateTime<Utc>,
    /// Capture is paused.
    pub paused: bool,
    /// What the lanes have heard.
    pub speech: SpeechTotals,
    /// Recording health reports a full disk.
    pub disk_full: bool,
    /// The calendar event the meeting was started for, if any.
    pub event: Option<CalendarEventId>,
}

/// The live recorder.
pub trait RecordingProbe: Send + Sync {
    /// The running (or paused) recording; `None` while idle, starting or
    /// stopping.
    fn recording(&self) -> Option<RecordingView>;
}

/// Who holds the microphone, from meeting detection.
pub trait MicrophoneHolders: Send + Sync {
    /// Whether `app` is capturing from the microphone now.
    fn holds(&self, app: &BundleId) -> bool;
    /// The meeting app detection sees now, if any (an ad-hoc recording
    /// follows it).
    fn meeting_app(&self) -> Option<BundleId>;
}

/// The synced calendar around now.
pub trait CalendarWindows: Send + Sync {
    /// The events near `now` (an hour before to two hours after), without
    /// all-day or declined ones.
    fn events_near(&self, now: DateTime<Utc>) -> Vec<EventWindow>;
}

/// The user's auto-stop settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoStopConfig {
    /// End recordings on their own (mic released, no speech, sleep).
    pub enabled: bool,
    /// Also end them once the calendar event is over and it went quiet.
    pub calendar_end: bool,
    /// Offer a new note when the next meeting begins.
    pub back_to_back: bool,
}

impl Default for AutoStopConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            calendar_end: false,
            back_to_back: true,
        }
    }
}

/// Reads the settings on every tick.
pub trait AutoStopSettings: Send + Sync {
    /// The settings now.
    fn config(&self) -> AutoStopConfig;
}

/// What auto-stop does to the app. Calls return quickly; stopping and
/// splitting run in the background.
pub trait AutoStopActions: Send + Sync {
    /// The notice or the split offer changed (shown, or withdrawn).
    fn changed(&self, state: &AutoStopState);
    /// Stops the recording.
    fn stop(&self, reason: StopReason);
    /// Stops the recording and starts a new one for `event`.
    fn split(&self, event: &EventWindow);
}
