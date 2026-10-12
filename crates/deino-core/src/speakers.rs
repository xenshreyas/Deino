//! Speaker naming (CAP-04): the participants panel, renaming a speaker for
//! one segment or all of them, merging two speakers, and the suggestions
//! `identify_speakers` leaves for the user.
//!
//! - [`SpeakerService`][]: [`participants`](SpeakerService::participants)
//!   (colour, name, speaking time and share, suggestion),
//!   [`candidates`](SpeakerService::candidates) (attendees first, then
//!   people), [`rename`](SpeakerService::rename) (see [`rename`]),
//!   [`merge`](SpeakerService::merge), and confirming or dismissing a
//!   suggestion.
//! - Ports: [`ParticipantReader`], [`NamingRepo`] (the user's writes) and
//!   [`IdentificationWriter`] (the stage's), all implemented by
//!   [`SqliteNamingRepo`], one transaction per change.
//!
//! **Propagation.** A speaker's name is its person's name, read wherever the
//! speaker is shown: the transcript, the notes input (`legacy_summary`
//! prefixes each line with it) and the panel. Naming a speaker moves the
//! meeting's action items from its previous person to the new one; the UI
//! then offers to regenerate the notes.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod naming_sql;
mod rename;
mod repo;
#[cfg(test)]
mod repo_tests;
mod seating_sql;
mod service;

pub use error::SpeakerError;
pub use model::{
    Identification, MAX_NAME_CHARS, MergeSpeakers, NameCandidate, NameSpeaker, Participant,
    ParticipantRow, PersonRef, Rename, RenameScope, Renamed, Seating, SegmentMove, SegmentTarget,
    SpeakerName, SuggestedName, Suggestion,
};
pub use repo::{IdentificationWriter, NamingRepo, ParticipantReader, SqliteNamingRepo};
pub use service::{CANDIDATE_LIMIT, SpeakerPorts, SpeakerService, share_percent};
