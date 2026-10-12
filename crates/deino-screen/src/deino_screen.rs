//! Screen capture (CAP-09): the meeting window's video and snapshots of
//! what is shared in it, through `ScreenCaptureKit`, while Deino records.
//!
//! ```text
//!   recording starts ─▶ plan::decide (settings, detected app, permission)
//!        │ CapturePlan
//!        ▼
//!   ScreenCapture::start ─▶ Capturer thread: pick_window ─▶ ScreenSource::open
//!        FrameFeed ─Frame─▶ Capture::step ◀─position─ RecordingClock
//!                              │ visible? (Windows, every 2 s)
//!                              ├─▶ VideoEncoder ─▶ <meeting>/video.part.mp4 ─stop─▶ video.mp4
//!                              └─▶ SnapshotSampler (10 s, PHash) ─▶ snapshots/<t>.png + SnapshotStore
//! ```
//!
//! - [`frame`]: [`Frame`] (owned BGRA) and [`FrameRate`];
//! - [`phash`], [`sampler`]: the perceptual hash and which frames are kept;
//! - [`encode`]: snapshots as PNG, written atomically;
//! - [`target`]: which window, at what size; [`plan`]: whether to capture;
//! - [`capture`], [`capturer`], [`service`]: one recording's capture, its
//!   thread, and the app's handle ([`ScreenCapture`]);
//! - [`ports`]: the platform, the clock and the store; `macos/` implements
//!   the platform, elsewhere [`platform`] is `None`.
//!
//! The video has no sound of its own: the `screen_video` pipeline stage
//! adds the meeting's mixed audio once `finalize_audio` has written it
//! (ADR 2026-10-12-screen-capture). Deino never asks for Screen Recording
//! on its own: a capture only starts when it is already granted, and
//! Settings opens the System Settings page. Nothing here logs window
//! titles or pixels, and nothing leaves the Mac.

pub mod capture;
pub mod capturer;
pub mod encode;
pub mod error;
#[cfg(any(test, feature = "test-support"))]
pub mod fake;
pub mod frame;
#[cfg(target_os = "macos")]
mod macos;
pub mod phash;
pub mod pick;
pub mod plan;
pub mod ports;
pub mod sampler;
pub mod service;
pub mod status;
pub mod target;

pub use capture::{SNAPSHOTS, VIDEO_FILE, VIDEO_PART, has_video, snapshot_name};
pub use error::ScreenError;
pub use frame::{Frame, FrameRate};
pub use phash::PHash;
pub use pick::{
    CaptureTarget, DisplayInfo, OWN_BUNDLE, Resolved, Source, TargetChoice, choices, resolve,
};
pub use plan::{CapturePlan, CaptureSettings, PickInput, PlanInput, Skip, decide, picked};
pub use ports::{
    FrameFeed, Platform, Position, RecordingClock, ScreenPermission, ScreenSource, SnapshotRecord,
    SnapshotStore, VideoEncoder, VideoEncoders, Windows,
};
pub use sampler::{SnapshotPolicy, SnapshotSampler, Verdict};
pub use service::{ScreenCapture, ScreenCaptureParts};
pub use status::{CaptureState, CaptureStatus};
pub use target::WindowInfo;

/// This Mac's screen ports: `ScreenCaptureKit`, `AVAssetWriter` and the
/// Screen Recording permission (read with `CGPreflightScreenCaptureAccess`,
/// which never prompts).
#[cfg(target_os = "macos")]
pub fn platform() -> Option<Platform> {
    Some(macos::platform())
}

/// No screen capture off macOS.
#[cfg(not(target_os = "macos"))]
pub fn platform() -> Option<Platform> {
    None
}

/// Asks macOS for Screen Recording, showing its prompt the first time;
/// true when granted. Only for the user's own click (onboarding, Settings):
/// a capture never asks.
#[cfg(target_os = "macos")]
pub fn request_screen_recording() -> bool {
    macos::request_permission()
}

/// Nothing to ask for off macOS.
#[cfg(not(target_os = "macos"))]
pub fn request_screen_recording() -> bool {
    false
}

/// The text Apple Vision (`VNRecognizeTextRequest`, accurate, English)
/// reads in the image file at `path`, one line per observation; `""` when
/// there is none. Blocking: run it off the async runtime.
#[cfg(target_os = "macos")]
pub fn recognize_text(path: &std::path::Path) -> Result<String, ScreenError> {
    macos::recognize_text(path)
}

/// No text recognition off macOS.
#[cfg(not(target_os = "macos"))]
pub fn recognize_text(_path: &std::path::Path) -> Result<String, ScreenError> {
    Err(ScreenError::Unsupported)
}

/// Opens System Settings → Privacy & Security → Screen Recording, where the
/// user allows Deino. Deino never changes the setting itself.
#[cfg(target_os = "macos")]
pub fn open_screen_recording_settings() -> Result<(), ScreenError> {
    macos::open_settings()
}

/// There is no such page off macOS.
#[cfg(not(target_os = "macos"))]
pub fn open_screen_recording_settings() -> Result<(), ScreenError> {
    Err(ScreenError::Unsupported)
}
