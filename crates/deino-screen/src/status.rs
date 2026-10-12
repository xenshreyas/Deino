//! [`CaptureStatus`]: what screen capture is doing now, for the record
//! panel's "Capturing" / "Paused screen capture" label and Settings.

use std::sync::{Mutex, PoisonError};

use deino_domain::MeetingId;
use serde::Serialize;

/// What the capture of the current recording is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum CaptureState {
    /// Not capturing: off, not a detected meeting, or no recording.
    #[default]
    Off,
    /// Starting: finding the window and opening the stream.
    Starting,
    /// Capturing the meeting window.
    Capturing,
    /// Paused: the meeting window is minimised, hidden or closed.
    Hidden,
    /// Paused with the recording.
    Paused,
}

/// The capture's state, for the UI.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    /// The meeting being captured, if any.
    pub meeting: Option<MeetingId>,
    /// What the capture is doing.
    pub state: CaptureState,
    /// Whether the window is being recorded as a video.
    pub video: bool,
    /// Snapshots kept so far.
    pub snapshots: u32,
}

/// The shared [`CaptureStatus`], written by the capture thread.
#[derive(Debug, Default)]
pub struct StatusCell(Mutex<CaptureStatus>);

impl StatusCell {
    /// The status now.
    pub fn get(&self) -> CaptureStatus {
        self.lock().clone()
    }

    /// A capture of `meeting` is starting.
    pub fn begin(&self, meeting: &MeetingId, video: bool) {
        *self.lock() = CaptureStatus {
            meeting: Some(meeting.clone()),
            state: CaptureState::Starting,
            video,
            snapshots: 0,
        };
    }

    /// The capture of `meeting` is now in `state`; ignored for another meeting.
    pub fn set(&self, meeting: &MeetingId, state: CaptureState) {
        let mut status = self.lock();
        if status.meeting.as_ref() == Some(meeting) {
            status.state = state;
        }
    }

    /// The video of `meeting` stopped (it failed to open or write).
    pub fn video_stopped(&self, meeting: &MeetingId) {
        let mut status = self.lock();
        if status.meeting.as_ref() == Some(meeting) {
            status.video = false;
        }
    }

    /// One more snapshot of `meeting` was kept.
    pub fn snapshot_kept(&self, meeting: &MeetingId) {
        let mut status = self.lock();
        if status.meeting.as_ref() == Some(meeting) {
            status.snapshots = status.snapshots.saturating_add(1);
        }
    }

    /// The capture of `meeting` ended.
    pub fn end(&self, meeting: &MeetingId) {
        let mut status = self.lock();
        if status.meeting.as_ref() == Some(meeting) {
            *status = CaptureStatus::default();
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, CaptureStatus> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capture_reports_its_state_until_it_ends() {
        let cell = StatusCell::default();
        let meeting = MeetingId::new("meeting");
        cell.begin(&meeting, true);
        cell.set(&meeting, CaptureState::Hidden);
        cell.snapshot_kept(&meeting);
        cell.set(&MeetingId::new("other"), CaptureState::Capturing);

        let status = cell.get();
        assert_eq!(status.state, CaptureState::Hidden);
        assert_eq!(status.snapshots, 1);

        cell.end(&meeting);
        assert_eq!(cell.get(), CaptureStatus::default());
    }
}
