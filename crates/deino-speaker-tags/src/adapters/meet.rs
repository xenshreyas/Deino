//! [`MeetAdapter`]: Google Meet in Google Chrome (`com.google.Chrome`).
//!
//! A Chrome window's `AXDocument` is its active tab's address, so a window
//! may hold a call when that tab is a Meet room (`meet.google.com/abc-defg-hij`),
//! and does once the room's "Leave call" button exists (the lobby has
//! none). Chrome only exposes web content to Accessibility when asked: the
//! watcher sets Chromium's `AXManualAccessibility` on Chrome once per
//! process (`macos/`). A Meet tab in the background of its window has no
//! tree; the tab must be the window's active one.
//!
//! Each tile's name is a `span.notranslate`, and its voice indicator
//! carries one of Meet's speaking classes while the participant speaks.
//! The classes are Meet's generated names, taken from the capture and from
//! anarlog's Meet worker (`THIRD_PARTY_NOTICES.md`); they change with
//! Meet's releases, which the adapter reports as an unrecognised layout.

use deino_domain::CallApp;

use super::classed::{Classes, scan};
use crate::adapter::SpeakerAdapter;
use crate::reading::{ReadError, Reading};
use crate::tree::{AxNode, WindowSummary};

/// Meet's host.
const MEET_HOST: &str = "meet.google.com/";

/// The call's hang-up button.
const LEAVE_BUTTON: &str = "Leave call";

/// Meet's classes.
const CLASSES: Classes = Classes {
    name: &["notranslate"],
    indicator: &["IisKdb", "DYfzY"],
    speaking: &[
        "Oaajhc",
        "HX2H7",
        "wEsLMd",
        "OgVli",
        "speaking",
        "active-speaker",
        "speaker-active",
        "speaking-indicator",
    ],
};

/// Reads Meet's call tab in Chrome.
#[derive(Clone, Copy, Debug, Default)]
pub struct MeetAdapter;

impl SpeakerAdapter for MeetAdapter {
    fn app(&self) -> CallApp {
        CallApp::Meet
    }

    fn bundle_ids(&self) -> &'static [&'static str] {
        &["com.google.Chrome"]
    }

    fn is_call_window(&self, window: &WindowSummary) -> bool {
        window.document.as_deref().is_some_and(is_meeting_room)
    }

    fn read(&self, window: &AxNode) -> Result<Reading, ReadError> {
        if !self.is_call_window(&window.summary()) || !window.has_button(LEAVE_BUTTON) {
            return Err(ReadError::NotInCall);
        }
        scan(window, &CLASSES).into_reading()
    }
}

/// Whether `address` is a Meet room: `meet.google.com/` and a code of
/// three, four and three lower-case letters.
fn is_meeting_room(address: &str) -> bool {
    let rest = address
        .strip_prefix("https://")
        .or_else(|| address.strip_prefix("http://"))
        .unwrap_or(address);
    let Some(path) = rest.strip_prefix(MEET_HOST) else {
        return false;
    };
    let code = path.split(['?', '#', '/']).next().unwrap_or_default();
    let groups: Vec<&str> = code.split('-').collect();
    let lengths: Vec<usize> = groups.iter().map(|group| group.len()).collect();
    lengths == [3, 4, 3]
        && groups
            .iter()
            .all(|group| group.chars().all(|c| c.is_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_meet_rooms_are_call_windows() {
        for room in [
            "https://meet.google.com/abc-defg-hij",
            "https://meet.google.com/abc-defg-hij?authuser=1",
            "meet.google.com/xyz-abcd-efg",
        ] {
            assert!(is_meeting_room(room), "{room}");
        }
        for other in [
            "https://meet.google.com/",
            "https://meet.google.com/landing",
            "https://example.com/meet.google.com/abc-defg-hij",
            "https://meet.google.com/ABC-defg-hij",
        ] {
            assert!(!is_meeting_room(other), "{other}");
        }
    }
}
