//! Meetings: listing, opening and renaming the meetings the user has
//! recorded, and the Meetings page's list, views, trash, bulk actions and
//! merge (ORG-07).
//!
//! [`MeetingService`] holds the rules (titles are trimmed, non-empty and
//! bounded; renames are stamped by the injected `Clock`). It reaches storage
//! only through the [`MeetingRepo`] port, implemented by [`SqliteMeetingRepo`]
//! in the app and by `FakeMeetingRepo` in tests.
//!
//! FND-07 added the lifecycle: [`MeetingStatus`](deino_domain::MeetingStatus)
//! moves only through [`MeetingLifecycleService::transition`], and
//! [`MeetingLifecycleRepo`] stores timing, recording files and calendar
//! links ([`MeetingState`]). [`MeetingTrashRepo`] moves single meetings into
//! and out of the trash.
//!
//! ORG-07 adds, each a port [`SqliteMeetingRepo`] implements and a service:
//!
//! | Module | Port | Service |
//! |---|---|---|
//! | [`list`] | [`MeetingListRepo`] | [`MeetingListService`]: filtered cursor pages, ⌘A |
//! | `views` | [`MeetingViewRepo`] | [`MeetingViewService`]: saved views as tabs |
//! | [`trash`] | [`TrashBin`], [`MeetingFiles`] | [`TrashService`], [`TrashPurger`] |
//! | [`merge`] | [`MeetingMergeRepo`] | [`MeetingMergeService`] |
//! | `bulk` | the tag and folder services | [`MeetingBulkService`] |
//!
//! The ports stay separate so each caller depends only on what it uses.
//! The trash and the merge reach the post-meeting pipeline through
//! [`MeetingWork`]: trashing cancels a meeting's running stage, a merge
//! re-indexes the merged meeting.
//! Features that react to the lifecycle implement [`MeetingStatusObserver`]
//! and subscribe to the service (FND-08's pipeline starts on `processing`).

mod bulk;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod lifecycle;
mod lifecycle_repo;
#[cfg(test)]
mod lifecycle_repo_tests;
mod lifecycle_service;
pub mod list;
pub mod merge;
mod model;
mod observer;
mod repo;
mod service;
#[cfg(test)]
mod test_seed;
pub mod trash;
mod trash_repo;
#[cfg(test)]
mod trash_repo_tests;
mod view_service;
mod views;
mod work;

pub use bulk::{BulkError, BulkOutcome, MeetingBulkService};
pub use error::MeetingError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeMeetingRepo;
pub use lifecycle::{
    MeetingAudio, MeetingLink, MeetingState, MeetingTiming, NewMeeting, StatusChange,
};
pub use lifecycle_repo::MeetingLifecycleRepo;
pub use lifecycle_service::MeetingLifecycleService;
pub use list::{
    DateRange, ListedAttendee, ListedMeeting, ListedTag, MeetingCursor, MeetingFilter,
    MeetingListDeps, MeetingListRepo, MeetingListService, MeetingPage, OwnAddresses,
};
pub use merge::{MeetingMergeRepo, MeetingMergeService, MergeDeps, MergeOutcome};
pub use model::{MAX_TITLE_CHARS, Meeting, MeetingTitle};
pub use observer::MeetingStatusObserver;
pub use repo::{MeetingRepo, SqliteMeetingRepo};
pub use service::MeetingService;
pub use trash::{
    DiskMeetingFiles, MeetingFiles, TrashBin, TrashDeps, TrashOutcome, TrashPurger, TrashService,
    TrashedMeeting,
};
pub use trash_repo::MeetingTrashRepo;
pub use view_service::{MAX_VIEW_NAME_CHARS, MeetingViewService};
pub use views::{MeetingView, MeetingViewInput, MeetingViewRepo};
#[cfg(any(test, feature = "test-support"))]
pub use work::RecordedWork;
pub use work::{MeetingWork, NoMeetingWork};
