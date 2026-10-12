//! Consent tools (CAP-13): the automatic notice Deino posts in the meeting
//! chat, so the people on a call know it is transcribed.
//!
//! ```text
//!   ChatNotice thread, every second:
//!     Recorder ─(meeting, transcribing?, speech heard?)─▶ Monitor ─▶ Session (one per meeting)
//!     NoticeConfig (Settings → Consent: message, per-app switches) ──┘        │
//!     CAP-05 WindowReader ─AxNode─▶ ChatAdapter::view ─ChatView──────────────┤
//!     KeyActivity (last key press) ──────────────────────────────▶ decide ──┤
//!                                                    Decision::Post ─▶ attempt::post ─▶ ChatDriver
//! ```
//!
//! - [`path`]: where an element sits under its window;
//! - [`chat`] and [`apps`]: one [`ChatAdapter`] per app (Zoom, Teams, Meet
//!   in Chrome) over CAP-05's [`SpeakerAdapter`](deino_speaker_tags::SpeakerAdapter),
//!   unit-tested on fixture dumps;
//! - [`gate`]: when to post ([`decide`], pure): transcribing, speech heard,
//!   an empty composer, and no key press for [`QUIET`];
//! - [`attempt`]: one post, re-checking the typing guard between steps;
//! - [`session`], [`monitor`]: once per meeting, and the polling thread;
//! - [`ports`]: the driver, the keyboard, the recorder and the settings;
//!   `macos/` implements the first two, elsewhere [`platform`] is `None`.
//!
//! Off by default. Everything degrades without a sound: no permission, no
//! call, an unknown layout or a failing write are typed [`ChatError`]s
//! logged once per meeting at debug level, and the notice is not posted.
//! Deino never posts while the user types, never writes into a composer
//! that holds text, and never posts twice in one meeting. Nothing here logs
//! the message, names, titles or addresses, and nothing leaves the Mac
//! except the message the user wrote, typed into the meeting app's chat.

pub mod apps;
pub mod attempt;
pub mod chat;
pub mod error;
#[cfg(any(test, feature = "test-support"))]
pub mod fake;
pub mod gate;
#[cfg(target_os = "macos")]
mod macos;
pub mod monitor;
pub mod path;
pub mod ports;
pub mod session;

pub use apps::{MeetChat, TeamsChat, ZoomChat, chat_adapter_for};
pub use attempt::{Outcome, Post, Retry, post};
pub use chat::{ChatAdapter, ChatView, ChatVocabulary, Composer};
pub use error::{ChatError, LoggedOnce};
pub use gate::{Decision, Hold, Moment, QUIET, decide, is_quiet};
pub use monitor::{ChatNotice, Monitor, NoticeParts, POLL_INTERVAL};
pub use path::NodePath;
pub use ports::{
    ChatConfig, ChatDriver, ConsentPlatform, KeyActivity, NoticeConfig, Recorder, RecorderNow,
    Target,
};
pub use session::{MAX_FAILURES, Ports, Progress, Session};

/// This Mac's ports: CAP-05's window reader, the chat driver and the
/// keyboard.
#[cfg(target_os = "macos")]
pub fn platform() -> Option<ConsentPlatform> {
    macos::platform()
}

/// The chat notice needs macOS Accessibility; elsewhere there is no platform.
#[cfg(not(target_os = "macos"))]
pub fn platform() -> Option<ConsentPlatform> {
    None
}
