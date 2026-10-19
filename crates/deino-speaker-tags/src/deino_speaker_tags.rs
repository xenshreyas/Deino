//! Speaker tags (CAP-05): who the meeting app shows as the active speaker,
//! read through macOS Accessibility while Deino records.
//!
//! ```text
//!   recording starts ─▶ SpeakerTags::watch ─▶ Watcher thread, every 250 ms:
//!       RecordingClock ─position─▶ Watch::tick ─▶ AccessibilityPermission?
//!                                     │          RunningApps (Zoom, Teams, Chrome)
//!                                     │          WindowReader ─AxNode─▶ SpeakerAdapter
//!                                     ▼                                   │ Reading
//!                                  Timeline ◀─────────── who speaks ──────┘
//!                                     │ TagSpan (t ms on the recording's clock)
//!                                     ▼
//!                                  TagSink ─▶ speaker_tag_spans ─▶ identify_speakers
//! ```
//!
//! - [`tree`]: [`AxNode`], the snapshot every adapter reads, live or from a
//!   fixture dump;
//! - [`adapter`] and `adapters/`: one [`SpeakerAdapter`] per app (Zoom,
//!   Teams, Meet in Chrome), each unit-tested on fixture dumps;
//! - [`reading`], [`label`]: what a window says, names without decorations;
//! - [`timeline`]: polls into [`TagSpan`](deino_domain::TagSpan)s;
//! - [`watch`], [`watcher`], [`service`]: one recording's polling, its
//!   thread, and the app's handle ([`SpeakerTags`]);
//! - [`capture`]: the opt-in dump of a call window that cannot be read;
//! - [`status`]: Settings → Speaker tags (Enabled, Needs permission,
//!   Unsupported);
//! - [`ports`]: the platform, the clock and the sink; `macos/` implements
//!   the platform, elsewhere [`platform`] is `None`.
//!
//! Everything degrades to voice matching without a sound: no permission,
//! no supported app, an unknown window layout or a failing read are typed
//! [`TagError`]s logged once at debug level. Deino never asks for
//! Accessibility on its own and never changes a system setting; Settings
//! opens the System Settings page and the user decides. Nothing here logs
//! names, titles or addresses, and nothing leaves the Mac.

pub mod adapter;
mod adapters;
pub mod capture;
pub mod error;
#[cfg(any(test, feature = "test-support"))]
pub mod fake;
pub mod label;
#[cfg(target_os = "macos")]
mod macos;
mod outbox;
pub mod ports;
pub mod presence;
pub mod reading;
pub mod service;
pub mod status;
pub mod timeline;
pub mod tree;
pub mod watch;
pub mod watcher;

pub use adapter::{SpeakerAdapter, adapter_for};
pub use adapters::{MeetAdapter, TeamsAdapter, ZoomAdapter};
pub use capture::{CAPTURE_AFTER, NoCapture, WindowCapture};
pub use error::{AxError, SinkError, TagError};
pub use label::Label;
pub use ports::{
    AccessibilityPermission, Pid, Platform, Position, RecordingClock, RunningApps, TagSink,
    WindowReader,
};
pub use presence::Presence;
pub use reading::{Active, ReadError, Reading};
pub use service::{SpeakerTags, SpeakerTagsParts};
pub use status::{AppTagStatus, LayoutHealth, TagState};
pub use timeline::{POLL_INTERVAL, Timeline};
pub use tree::{AxNode, Frame, WindowSummary};
pub use watch::{Step, Watch, WatchParts};
pub use watcher::Watcher;

/// This Mac's Accessibility ports.
#[cfg(target_os = "macos")]
pub fn platform() -> Option<Platform> {
    Some(macos::platform())
}

/// Speaker tags need macOS Accessibility; elsewhere there is no platform.
#[cfg(not(target_os = "macos"))]
pub fn platform() -> Option<Platform> {
    None
}

/// Opens System Settings → Privacy & Security → Accessibility, where the
/// user can add Deino. Never changes the setting itself.
#[cfg(target_os = "macos")]
pub fn open_accessibility_settings() -> Result<(), AxError> {
    if macos::open_accessibility_settings() {
        Ok(())
    } else {
        Err(AxError::SettingsUnavailable)
    }
}

/// There is no Accessibility page off macOS.
#[cfg(not(target_os = "macos"))]
pub fn open_accessibility_settings() -> Result<(), AxError> {
    Err(AxError::Unsupported)
}

/// Every window of `app`'s running processes, with every attribute a dump
/// keeps: the capture tool behind `fixtures/README.md`. Needs Accessibility
/// for the calling process (the terminal); it never prompts.
#[cfg(target_os = "macos")]
pub fn capture(app: deino_domain::CallApp) -> Result<Vec<AxNode>, AxError> {
    let adapter = adapter_for(app);
    let platform = macos::platform();
    let mut windows = Vec::new();
    for pid in platform.apps.pids(adapter.bundle_ids()) {
        platform.windows.prepare(app, pid);
        windows.extend(platform.windows.full_windows(pid, &|_| true)?);
    }
    Ok(windows)
}
