//! Voice profiles (CAP-04): confirmed exemplars of people's voices, so a
//! later meeting can name its speakers by ear.
//!
//! - [`Voiceprint`]: one diarized speaker's centroid (WeSpeaker ResNet34-LM,
//!   [`VOICEPRINT_DIM`] values) that the user confirmed by naming the
//!   speaker for all of its segments, with the [`Quality`] of the speech
//!   behind it ([`quality`]). A person keeps their best
//!   [`MAX_VOICEPRINTS_PER_PERSON`].
//! - [`VoiceprintService`]: enrols ([`VoiceprintService::enrol`], called only
//!   on a confirmed rename) and lists and deletes [`VoiceProfile`]s for
//!   Settings → People → Voice profiles.
//! - [`ExemplarReader`]: what matching reads; [`matching`] is how
//!   (`pick_unique`, `complete_by_elimination`, the max over exemplars),
//!   used by `diarization::identify` and the engine's known-speaker merge.
//! - [`spans`]: the clean single-speaker spans that measure quality.
//!
//! [`SqliteVoiceprintRepo`] implements the ports over the `voiceprints`
//! table; `FakeVoiceprintRepo` stands in for tests.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
pub mod matching;
mod model;
pub mod quality;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;
pub mod spans;

pub use error::VoiceprintError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeVoiceprintRepo;
pub use model::{
    Exemplar, MAX_VOICEPRINTS_PER_PERSON, ProfileRow, Quality, VOICEPRINT_DIM, VoiceEmbedding,
    VoiceProfile, Voiceprint, VoiceprintSummary, group_profiles,
};
pub use repo::{ExemplarReader, SqliteVoiceprintRepo, VoiceprintRepo};
pub use service::{EnrolSkip, Enrolment, MIN_ENROL_SPEECH, VoiceprintDeps, VoiceprintService};
