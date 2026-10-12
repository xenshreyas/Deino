//! Merging meetings (ORG-07; Meetily community request #393): two or more
//! meetings become one, the earliest, which keeps its ID.
//!
//! - **Transcript:** concatenated in time order; each later meeting's lines
//!   (and word timings) shift by the length of everything before it
//!   ([`timeline`]). Speakers move with them; the user and people the
//!   target already has a speaker for are joined.
//! - **People:** attendance is united with the stronger role, as the person
//!   merge does (ADR 0036).
//! - **Tags, folders, action items, voiceprint sources:** united or moved.
//! - **Notes:** each part's under a heading with its title ([`combine`]).
//! - **Recordings:** each absorbed meeting leaves a `meeting_merge_parts`
//!   row with its files and offset, so its audio stays reachable; its
//!   folders stay on disk until the merged meeting is deleted for good.
//! - **AI notes:** the target's versions stay; their basis no longer
//!   matches the transcript, so the meeting page offers to regenerate with
//!   NOT-02's typed reasons (`TranscriptEdited`, `AudioAppended`,
//!   `AttendeesChanged`).
//!
//! Everything is applied in one transaction ([`MeetingMergeRepo`]).

mod model;
mod notes;
mod plan;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;
mod steps;

pub use model::{MergeCandidate, MergeOutcome, MergePart, MergePlan};
pub use notes::{PartNotes, combine};
pub use plan::{Timeline, timeline};
pub use repo::MeetingMergeRepo;
pub use service::{MeetingMergeService, MergeDeps};
