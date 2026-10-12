//! The trash (ORG-07, Granola §3.6): soft delete with `deleted_at`, the
//! Trash view, restore, delete for good, and the 30-day purge that also
//! deletes each meeting's recordings.
//!
//! Trashing touches only `meetings.deleted_at`, so restoring brings back
//! everything. Readers skip trashed meetings themselves: the list, search
//! (ADR 0032: filtered at query time, so restoring needs no reindex),
//! exports, the CLI's read layer (`MeetingRepo::find` and `list` see only
//! meetings outside the trash), related meetings and people's "last met".
//!
//! [`TrashBin`] is the batch port, [`MeetingFiles`] deletes files
//! ([`DiskMeetingFiles`] in the app), [`TrashService`] holds the rules and
//! [`TrashPurger`] runs the purge.

mod files;
mod model;
mod purge;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;

pub use files::{DiskMeetingFiles, MeetingFiles};
pub use model::{StoredFiles, TRASH_RETENTION, TrashOutcome, TrashedMeeting};
pub use purge::{Housekeeping, PURGE_INTERVAL, TrashPurger};
pub use repo::TrashBin;
pub use service::{TrashDeps, TrashService};
