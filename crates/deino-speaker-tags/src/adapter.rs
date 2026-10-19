//! [`SpeakerAdapter`]: one meeting app's window, read. One implementation
//! per app in `adapters/`, chosen by [`adapter_for`]; the watcher and the
//! tests only ever see the trait.

use deino_domain::CallApp;

use crate::adapters::{MeetAdapter, TeamsAdapter, ZoomAdapter};
use crate::reading::{ReadError, Reading};
use crate::tree::{AxNode, WindowSummary};

/// Reads who is in a call and who is speaking from one app's windows.
///
/// Contract (every adapter's tests check it on fixture dumps):
/// - [`is_call_window`](Self::is_call_window) is cheap and may accept
///   windows that turn out to hold no call; it never rejects a call window;
/// - [`read`](Self::read) is pure, never panics, and fails with
///   [`ReadError::NotInCall`] for a window without a call and
///   [`ReadError::UnrecognisedLayout`] for a call window it cannot read;
/// - names are cleaned ([`crate::label::Label`]) and the user's own tile is
///   never a remote speaker.
pub trait SpeakerAdapter: Send + Sync {
    /// The app this adapter reads.
    fn app(&self) -> CallApp;

    /// The bundle identifiers the app runs under.
    fn bundle_ids(&self) -> &'static [&'static str];

    /// Whether `window` may hold a call, from its title and address alone.
    fn is_call_window(&self, window: &WindowSummary) -> bool;

    /// Reads a call window's full tree.
    fn read(&self, window: &AxNode) -> Result<Reading, ReadError>;

    /// Whether `window`, never a call itself, says something about the call
    /// (Zoom's main window names the signed-in user). Such windows are read
    /// along with the call windows and passed to
    /// [`read_with`](Self::read_with). None by default.
    fn is_context_window(&self, _window: &WindowSummary) -> bool {
        false
    }

    /// Reads a call window with the app's context windows read alongside;
    /// by default the context is not needed.
    fn read_with(&self, window: &AxNode, _context: &[AxNode]) -> Result<Reading, ReadError> {
        self.read(window)
    }
}

/// The adapter for `app`.
pub fn adapter_for(app: CallApp) -> &'static dyn SpeakerAdapter {
    match app {
        CallApp::Zoom => &ZoomAdapter,
        CallApp::Teams => &TeamsAdapter,
        CallApp::Meet => &MeetAdapter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_app_has_its_own_adapter_and_bundles() {
        for app in CallApp::ALL {
            let adapter = adapter_for(*app);
            assert_eq!(adapter.app(), *app);
            assert!(!adapter.bundle_ids().is_empty());
        }
    }
}
