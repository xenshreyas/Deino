//! Meeting detection (CAP-06): notice that a call started, and offer to take
//! notes for it.
//!
//! The flow, one evaluation at a time on the [`Detector`] thread:
//!
//! ```text
//! OS listeners ─wake─▶ SignalReader ─SignalSnapshot─▶ resolve ─Candidate─▶ DetectionPolicy
//! (Core Audio processes,  (attribution,               (Muesli's    (dwell, cooldown,
//!  default input, CMIO     browser URLs,               priority)    handled, recording)
//!  cameras)                calendar)                                    │
//!                                                                       ▼
//!                       DetectionSink ◀── DetectionEvent ── DetectionService ──▶ RecordingStarter
//! ```
//!
//! - [`ports`]: the OS ports ([`Sensor`], [`AppDirectory`], [`BrowserTabs`]);
//!   `macos/` implements them with Core Audio process objects and their
//!   listeners, CoreMediaIO, `NSWorkspace` and Accessibility; elsewhere
//!   [`platform_sensors`] reports [`DetectError::Unsupported`].
//! - [`attribution`], [`classify`], [`url`]: who holds the microphone and
//!   what that means (pure, table-tested against real bundle IDs).
//! - [`resolver`] and [`policy`]: the decision, pure over a `Clock`.
//! - [`service`]: the rules around the policy and the prompt's buttons.
//! - [`outputs`] and [`calendar`]: the ports the app implements.
//! - [`autostop`]: CAP-07, ending a recording when its meeting is over and
//!   offering a new note when the next one begins.
//!
//! Detection reads no audio and sends nothing anywhere. It logs prompt ids
//! and counts, never window titles, URLs or calendar titles.

pub mod attribution;
pub mod autostop;
pub mod calendar;
pub mod classify;
mod detector;
mod error;
mod event;
#[cfg(any(test, feature = "test-support"))]
pub mod fake;
#[cfg(target_os = "macos")]
mod macos;
mod model;
pub mod outputs;
pub mod policy;
pub mod ports;
mod reader;
pub mod resolver;
mod service;
mod snapshot;
#[cfg(test)]
mod testing;
pub mod url;

pub use calendar::{CalendarLookup, LinkedEvent, NoCalendar};
pub use detector::{Detector, DetectorParts, FALLBACK_POLL, HEARTBEAT};
pub use error::DetectError;
pub use event::{Detection, DetectionEvent, DetectionId};
pub use model::{AppIdentity, AudioProcess, MeetingKind, MicClient, Pid, Platform};
pub use outputs::{
    ConfigSource, DetectionConfig, DetectionSink, IgnoreList, RecordingRequest, RecordingStarter,
    RecordingStatus,
};
pub use ports::{AppDirectory, BrowserTabs, RunningApp, Sensor, Sensors, Waker, Watch};
pub use reader::SignalReader;
pub use service::{DetectionPorts, DetectionService};
pub use snapshot::{BrowserMeeting, Candidate, CandidateKey, Dwell, SignalSnapshot};

/// The sensors of this Mac: Core Audio process objects, the default input
/// device, cameras, running apps and browser windows.
#[cfg(target_os = "macos")]
pub fn platform_sensors() -> Result<Sensors, DetectError> {
    Ok(macos::sensors())
}

/// Meeting detection needs macOS; other platforms report it unsupported.
#[cfg(not(target_os = "macos"))]
pub fn platform_sensors() -> Result<Sensors, DetectError> {
    Err(DetectError::Unsupported)
}

/// Whether meeting detection works on this platform.
pub const fn is_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Whether Accessibility is granted, so browser meetings are recognised by
/// URL. Without it, browser calls are detected after a sustained hold.
#[cfg(target_os = "macos")]
pub fn reads_browser_urls() -> bool {
    macos::reads_browser_urls()
}

/// Whether Accessibility is granted (never, off macOS).
#[cfg(not(target_os = "macos"))]
pub fn reads_browser_urls() -> bool {
    false
}
