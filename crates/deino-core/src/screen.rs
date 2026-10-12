//! Screen capture's stored state (CAP-09): the snapshots kept while a
//! meeting records, and the per-meeting off switch.
//!
//! `deino-screen` captures; the app's snapshot store appends each kept
//! picture through [`ScreenRepo::add_snapshot`], and "Turn off for this
//! meeting" writes [`ScreenRepo::set_capture`], which the next start (a
//! resumed recording) reads with [`ScreenRepo::capture_choice`]. The
//! `screen_context` stage (OCR, phase B) and the meeting page's Images
//! strip read [`ScreenRepo::snapshots`].
//!
//! [`ScreenService`] holds the rules (turning a meeting off stops its
//! capture and keeps it off) over [`ScreenRepo`] ([`SqliteScreenRepo`];
//! `FakeScreenRepo` for tests) and [`CaptureControl`], the running capture.

#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod repo;
mod service;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeScreenRepo;
pub use model::{NewSnapshot, Snapshot};
pub use repo::{ScreenRepo, SqliteScreenRepo};
pub use service::{CaptureControl, ScreenService};
