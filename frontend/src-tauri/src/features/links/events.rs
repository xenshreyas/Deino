//! The `links` plugin's typed events.

use serde::Serialize;
use tauri_specta::Event;

/// A link or hotkey queued a navigation: the window should take it
/// (`plugin:links|take`). Carries nothing, so it reveals nothing while
/// Deino is locked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type, Event)]
pub struct NavigationWaiting {}

/// A `deino://` link Deino could not follow: its user-safe reason, never
/// the link itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
pub struct LinkRejected {
    /// Why, for a toast.
    pub message: String,
}
