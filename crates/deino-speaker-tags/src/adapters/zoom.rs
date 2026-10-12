//! [`ZoomAdapter`]: the Zoom Workplace desktop app (`us.zoom.xos`).
//!
//! The call runs in a window titled "Zoom Meeting" (or "Zoom Webinar").
//! Zoom describes each video tile and each row of the Participants panel as
//! "Name (Host, me), Computer audio unmuted, Video on", and adds "Talking"
//! while that participant's active-speaker frame is lit. The user's own
//! tile carries "(me)". The web client (zoom.us in a browser) is not
//! supported, as in Granola.

use deino_domain::CallApp;

use super::described::{Grammar, scan};
use crate::adapter::SpeakerAdapter;
use crate::reading::{ReadError, Reading};
use crate::tree::{AxNode, WindowSummary};

/// Titles of Zoom's call windows.
const CALL_WINDOW_TITLES: &[&str] = &["Zoom Meeting", "Zoom Webinar"];

/// How Zoom describes participants.
const GRAMMAR: Grammar = Grammar {
    roles: &["AXGroup", "AXRow", "AXCell"],
    presence: &[
        "video on",
        "video off",
        "host",
        "co-host",
        "guest",
        "attendee",
    ],
    audio: &[
        "audio muted",
        "audio unmuted",
        "muted",
        "unmuted",
        "no audio",
    ],
    speaking: &["talking", "speaking", "active speaker"],
};

/// Reads Zoom's call window.
#[derive(Clone, Copy, Debug, Default)]
pub struct ZoomAdapter;

impl SpeakerAdapter for ZoomAdapter {
    fn app(&self) -> CallApp {
        CallApp::Zoom
    }

    fn bundle_ids(&self) -> &'static [&'static str] {
        &["us.zoom.xos"]
    }

    fn is_call_window(&self, window: &WindowSummary) -> bool {
        window.title.as_deref().is_some_and(|title| {
            CALL_WINDOW_TITLES
                .iter()
                .any(|call| title.starts_with(call))
        })
    }

    fn read(&self, window: &AxNode) -> Result<Reading, ReadError> {
        if !self.is_call_window(&window.summary()) {
            return Err(ReadError::NotInCall);
        }
        scan(window, &GRAMMAR).into_reading()
    }
}
