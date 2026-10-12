//! `deino://` links and what they, and the global hotkeys, ask Deino to do
//! (DEV-05).
//!
//! | Link | Meaning |
//! |---|---|
//! | `deino://meeting/<id>[?t=<ms>]` | Open a meeting, at a moment of its recording |
//! | `deino://meetings/<id>` | The same; older exports (EXP-01) wrote this shape |
//! | `deino://new-note[?event=<id>]` | A new note, for a calendar event when given |
//! | `deino://record/start`, `deino://record/stop` | Start or stop recording (CAP-10's path) |
//! | `deino://search?q=<query>` | Search the library |
//! | `deino://recipe?data=<recipe>` | Import a recipe (Recipes is not built yet: "WIP") |
//!
//! - [`model`]: [`DeepLink`] (one validated link), [`Navigation`] (where the
//!   window goes) and [`Intent`] (what a link or hotkey asks for);
//! - [`parse`]: `DeepLink::parse`, which validates every route;
//! - [`ports`]: the recorder, the app lock and the window, as the app wires them;
//! - [`service`]: [`LinkService`], which records at once and holds
//!   navigations until the window takes them, never while Deino is locked;
//! - `fake`: in-memory ports behind `test-support`.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod parse;
#[cfg(test)]
mod parse_tests;
mod ports;
mod service;
#[cfg(test)]
mod service_tests;

pub use error::LinkError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeLock, FakeRecorder, FakeShell};
pub use model::{DeepLink, Intent, MAX_LINK_CHARS, Navigation, SCHEME};
pub use ports::{LockProbe, Recorder, RecorderError, Shell};
pub use service::{LinkService, MAX_PENDING};
