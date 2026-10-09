//! Meetings: listing, opening and renaming the meetings the user has recorded.
//!
//! [`MeetingService`] holds the rules (titles are trimmed, non-empty and
//! bounded; renames are stamped by the injected `Clock`). It reaches storage
//! only through the [`MeetingRepo`] port, implemented by [`SqliteMeetingRepo`]
//! in the app and by `FakeMeetingRepo` in tests.
//!
//! Creating and deleting meetings is still legacy code in the app crate
//! (`database/repositories/`) until the recording and organisation cards port it.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod repo;
mod service;

pub use error::MeetingError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeMeetingRepo;
pub use model::{MAX_TITLE_CHARS, Meeting, MeetingTitle};
pub use repo::{MeetingRepo, SqliteMeetingRepo};
pub use service::MeetingService;
