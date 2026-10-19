//! [`ZoomAdapter`]: the Zoom Workplace desktop app (`us.zoom.xos`), as real
//! captures show it (macOS, October 2026; `fixtures/zoom/real-*.json`).
//!
//! The call runs in a window titled "Zoom Meeting" (or "Zoom Webinar").
//! Each video tile is an `AXTabGroup` described as
//! "Name, Computer audio muted|unmuted[, Video off]", holding an image and
//! a button described with the name. **Zoom shows no talking indicator**,
//! so the speaker is inferred ([`tiles`]):
//! - Speaker View: the one tile far larger than every other is the active
//!   speaker's (Zoom enlarges whoever talks);
//! - otherwise, the one remote participant whose audio is unmuted;
//! - otherwise nobody.
//!
//! No tile marks the user. The "Zoom Workplace" window, read alongside as
//! context, has the account button "Zoom, Own Name, In a Zoom Meeting, Pro
//! account"; an open Participants panel says "Own Name (Host, me)". Either
//! names the user's own tile, which is never a remote participant. The web
//! client (zoom.us in a browser) is not supported, as in Granola.

mod tiles;

use deino_domain::CallApp;

use crate::adapter::SpeakerAdapter;
use crate::reading::{ReadError, Reading};
use crate::tree::{AxNode, WindowSummary};

/// Titles of Zoom's call windows.
const CALL_WINDOW_TITLES: &[&str] = &["Zoom Meeting", "Zoom Webinar"];

/// The title of Zoom's main window, which names the signed-in user.
const MAIN_WINDOW_TITLE: &str = "Zoom Workplace";

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

    fn is_context_window(&self, window: &WindowSummary) -> bool {
        window.title.as_deref().map(str::trim) == Some(MAIN_WINDOW_TITLE)
    }

    fn read(&self, window: &AxNode) -> Result<Reading, ReadError> {
        self.read_with(window, &[])
    }

    fn read_with(&self, window: &AxNode, context: &[AxNode]) -> Result<Reading, ReadError> {
        if !self.is_call_window(&window.summary()) {
            return Err(ReadError::NotInCall);
        }
        let tiles = tiles::tiles(window);
        if tiles.is_empty() {
            return Err(ReadError::UnrecognisedLayout);
        }
        let own = tiles::own_name(window, context);
        Ok(tiles::reading(&tiles, own.as_ref()))
    }
}
