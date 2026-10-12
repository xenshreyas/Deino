//! [`TeamsAdapter`]: the Microsoft Teams desktop app (new Teams,
//! `com.microsoft.teams2`; classic Teams' `com.microsoft.teams` too).
//!
//! Every Teams window is titled "… | Microsoft Teams", the chat as much as
//! the call, so a window holds a call when it has the call's "Leave"
//! button. Teams labels each video tile and each row of the People pane
//! "Name (Organizer), Unmuted, Video on", with "Speaking" while the tile's
//! voice ring is lit; the user's own tile says "(You)".

use deino_domain::CallApp;

use super::described::{Grammar, scan};
use crate::adapter::SpeakerAdapter;
use crate::reading::{ReadError, Reading};
use crate::tree::{AxNode, WindowSummary};

/// The end of every Teams window title.
const WINDOW_SUFFIX: &str = "Microsoft Teams";

/// The call's hang-up button, present only during a call.
const LEAVE_BUTTON: &str = "Leave";

/// How Teams describes participants.
const GRAMMAR: Grammar = Grammar {
    roles: &["AXGroup", "AXButton", "AXRow", "AXCell"],
    presence: &[
        "video on",
        "video off",
        "camera on",
        "camera off",
        "organizer",
        "presenter",
        "attendee",
        "external",
        "in this meeting",
    ],
    audio: &["muted", "unmuted", "mic off", "mic on"],
    speaking: &["speaking", "talking"],
};

/// Reads Teams' call window.
#[derive(Clone, Copy, Debug, Default)]
pub struct TeamsAdapter;

impl SpeakerAdapter for TeamsAdapter {
    fn app(&self) -> CallApp {
        CallApp::Teams
    }

    fn bundle_ids(&self) -> &'static [&'static str] {
        &["com.microsoft.teams2", "com.microsoft.teams"]
    }

    fn is_call_window(&self, window: &WindowSummary) -> bool {
        window
            .title
            .as_deref()
            .is_some_and(|title| title.trim_end().ends_with(WINDOW_SUFFIX))
    }

    fn read(&self, window: &AxNode) -> Result<Reading, ReadError> {
        if !self.is_call_window(&window.summary()) || !window.has_button(LEAVE_BUTTON) {
            return Err(ReadError::NotInCall);
        }
        scan(window, &GRAMMAR).into_reading()
    }
}
