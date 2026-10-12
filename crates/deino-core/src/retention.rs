//! Retention policies (PRV-01, Granola §10.2, Circleback §15): recordings,
//! transcripts and whole meetings are deleted a chosen number of days after
//! the meeting (Settings → Privacy & security, `privacy.retention`).
//!
//! [`RetentionService::sweep`] does it, on the trash purge's schedule
//! (ADR 0046's `TrashPurger`, which runs it as [`Housekeeping`] before each
//! purge):
//!
//! | Setting | What goes | What stays |
//! |---|---|---|
//! | audio | the files under `recordings/<id>` (clips too), the meeting's recording paths, its clips' rows | everything else; a file outside the app's folders (an import's original) is never touched |
//! | transcript | the segments, their search documents (FTS triggers, ADR 0032) and transcript embeddings | the notes, the AI notes; regenerating is refused |
//! | everything | the meeting moves to the trash | the 30-day purge deletes it for good |
//!
//! Rows go first and files after, as in the trash: a crash in between leaves
//! orphaned files, never a meeting pointing at missing audio. Only finished
//! meetings (`ready` or `failed`) outside the trash count; their age is
//! counted from `created_at`.
//!
//! [`RetentionStore`] is SQL, [`RecordingFiles`] deletes inside the app's
//! recordings folder ([`DiskMeetingFiles`](crate::meetings::DiskMeetingFiles)),
//! [`MeetingTrash`] is ORG-07's trash and [`RetentionPolicy`] reads the
//! settings.
//!
//! [`Housekeeping`]: crate::meetings::trash::Housekeeping

mod error;
mod model;
mod ports;
mod repo;
#[cfg(test)]
mod safety_tests;
mod service;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod test_world;

pub use error::RetentionError;
pub use model::{MeetingRetention, SweepOutcome, cutoff};
pub use ports::{MeetingTrash, RecordingFiles, RetentionPolicy};
pub use repo::{RetentionStore, SqliteRetentionStore};
pub use service::{RetentionDeps, RetentionService};
