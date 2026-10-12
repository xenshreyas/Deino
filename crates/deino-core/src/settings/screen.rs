//! The screen section: whether Deino captures the meeting window while it
//! records a detected meeting, and what it keeps (CAP-09).

use serde::{Deserialize, Serialize};

/// The video's frame rate when the file says nothing: one frame a second.
const DEFAULT_FPS: u8 = 1;

/// The screen section of `Settings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent switches the user toggles one by one in Settings → Screen capture"
)]
pub struct ScreenSettings {
    /// Capture the meeting window when a detected meeting is recorded.
    /// Off by default: it needs Screen Recording and the other side's
    /// consent.
    pub detected_meetings: bool,
    /// Record the window as a video beside the audio.
    pub video: bool,
    /// The video's frames per second (1–30; 1 suits a shared screen).
    pub video_fps: u8,
    /// Keep a snapshot when the shared screen changes (every 10 s at most).
    pub snapshots: bool,
    /// Describe each snapshot with Claude (`vision.describe`), sending the
    /// picture. Off by default: only text read on this Mac reaches the
    /// notes otherwise.
    pub describe_snapshots: bool,
    /// How many times the screen capture consent reminder has shown; it
    /// shows the first [`CONSENT_REMINDERS`] times a capture runs.
    pub consent_reminders_shown: u8,
}

/// How many captures show the consent reminder.
pub const CONSENT_REMINDERS: u8 = 3;

impl Default for ScreenSettings {
    fn default() -> Self {
        Self {
            detected_meetings: false,
            video: true,
            video_fps: DEFAULT_FPS,
            snapshots: true,
            describe_snapshots: false,
            consent_reminders_shown: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_capture_is_off_until_the_user_turns_it_on() {
        let missing: ScreenSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(missing, ScreenSettings::default());
        assert!(!missing.detected_meetings);
        assert!(missing.video && missing.snapshots);
        assert_eq!(missing.video_fps, 1);
        assert!(
            !missing.describe_snapshots,
            "pictures never leave the Mac by default"
        );
        assert_eq!(missing.consent_reminders_shown, 0);

        let on: ScreenSettings =
            serde_json::from_str(r#"{ "detectedMeetings": true, "videoFps": 5 }"#).unwrap();
        assert!(on.detected_meetings);
        assert_eq!(on.video_fps, 5);
    }
}
