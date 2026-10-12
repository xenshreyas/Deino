//! Auto-stop and meeting boundaries (CAP-07): ending a recording when the
//! meeting is over, and offering a new note when the next one begins.
//!
//! ```text
//!   ticker (1 s) ─▶ AutoStopService ──Observation──▶ AutoStopPolicy ─▶ Decision
//!                        │  ▲                                (notice, withdraw, stop)
//!     RecordingProbe ────┘  ├── MicrophoneHolders (DetectionService: who holds the mic)
//!     CalendarWindows ──────┘   boundary::next_meeting ─▶ SplitOffer
//!                        │
//!                        └──▶ AutoStopActions (the app: the panel's notice, stop, split)
//! ```
//!
//! - [`policy`]: the pure stop rules over a fake-able clock.
//! - [`boundary`]: the pure back-to-back rule: the next calendar event has
//!   begun and its attendees differ, so ask "Start new note for …?".
//! - [`service`]: the rules around them: which app the recording follows,
//!   speech since the last tick, the user's answers.
//! - [`ticker`]: the thread that ticks the service once a second.
//!
//! Like detection, it reads no audio and logs no titles.

pub mod boundary;
pub mod policy;
mod ports;
pub mod service;
pub mod ticker;

pub use boundary::EventWindow;
pub use policy::{AutoStopPolicy, Decision, EndingNotice, Heard, Observation, StopReason};
pub use ports::{
    AutoStopActions, AutoStopConfig, AutoStopSettings, CalendarWindows, MicrophoneHolders,
    RecordingProbe, RecordingView, SpeechTotals,
};
pub use service::{AutoStopService, AutoStopState, SplitOffer};
pub use ticker::AutoStopTicker;
