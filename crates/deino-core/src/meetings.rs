//! Meetings: listing, opening and renaming the meetings the user has recorded.
//!
//! [`MeetingService`] holds the rules (titles are trimmed, non-empty and
//! bounded; renames are stamped by the injected `Clock`). It reaches storage
//! only through the [`MeetingRepo`] port, implemented by [`SqliteMeetingRepo`]
//! in the app and by `FakeMeetingRepo` in tests.
//!
//! FND-07 added the lifecycle: [`MeetingStatus`](deino_domain::MeetingStatus)
//! moves only through [`MeetingLifecycleService::transition`], and
//! [`MeetingLifecycleRepo`] stores timing, recording files and calendar
//! links ([`MeetingState`]). [`MeetingTrashRepo`] moves meetings into and out
//! of the trash. One [`SqliteMeetingRepo`] implements all three ports; the
//! ports stay separate so each caller depends only on what it uses.
//! Features that react to the lifecycle implement [`MeetingStatusObserver`]
//! and subscribe to the service (FND-08's pipeline starts on `processing`).
//!
//! The legacy recording, import and delete paths in the app crate
//! (`database/repositories/`) still write meetings directly until FND-05b,
//! CAP-11 and ORG-07 move them onto these ports.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod lifecycle;
mod lifecycle_repo;
#[cfg(test)]
mod lifecycle_repo_tests;
mod lifecycle_service;
mod model;
mod observer;
mod repo;
mod service;
mod trash_repo;
#[cfg(test)]
mod trash_repo_tests;

pub use error::MeetingError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeMeetingRepo;
pub use lifecycle::{
    MeetingAudio, MeetingLink, MeetingState, MeetingTiming, NewMeeting, StatusChange,
};
pub use lifecycle_repo::MeetingLifecycleRepo;
pub use lifecycle_service::MeetingLifecycleService;
pub use model::{MAX_TITLE_CHARS, Meeting, MeetingTitle};
pub use observer::MeetingStatusObserver;
pub use repo::{MeetingRepo, SqliteMeetingRepo};
pub use service::MeetingService;
pub use trash_repo::MeetingTrashRepo;
