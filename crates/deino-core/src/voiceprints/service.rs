//! `VoiceprintService`: enrolment on a confirmed rename, and the voice
//! profiles Settings lists and deletes.
//!
//! **Only a confirmed "rename all segments" enrols** (the speaker naming
//! service calls [`VoiceprintService::enrol`]; nothing else does): the spike
//! saw one bad exemplar cause false accepts in every later meeting. A voice
//! with under [`MIN_ENROL_SPEECH`] of clean speech is a phantom or a
//! fragment, so it is skipped rather than enrolled.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId, Millis, PersonId, SpeakerId, VoiceprintId};

use super::quality::{clean_speech, quality_of};
use super::{
    MAX_VOICEPRINTS_PER_PERSON, VoiceEmbedding, VoiceProfile, Voiceprint, VoiceprintError,
    VoiceprintRepo, group_profiles,
};
use crate::segments::{Page, SegmentReader, SpeakerRepo};

/// Clean, overlap-free speech a voice needs before it is enrolled: two of
/// the spike's 1.5 s enrolment spans. Less is a phantom cluster's worth.
pub const MIN_ENROL_SPEECH: Millis = Millis(3_000);

/// The ports [`VoiceprintService`] is built from.
#[derive(Clone)]
pub struct VoiceprintDeps {
    /// Where voiceprints live.
    pub repo: Arc<dyn VoiceprintRepo>,
    /// Diarized speakers and their voices.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The transcript, for the voice's quality.
    pub segments: Arc<dyn SegmentReader>,
    /// IDs and enrolment times.
    pub clock: Arc<dyn Clock>,
}

/// What enrolling a speaker's voice did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Enrolment {
    /// The voice was saved; `evicted` weaker exemplars went to keep five.
    Enrolled {
        /// The new voiceprint.
        id: VoiceprintId,
        /// Exemplars removed to make room (the new one, if it was weakest).
        evicted: usize,
    },
    /// Nothing was saved, for this reason.
    Skipped(EnrolSkip),
}

/// Why a voice was not enrolled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnrolSkip {
    /// The speaker has no usable diarized voice (added by hand, or "Me").
    NoVoice,
    /// The voice is backed by too little clean speech.
    TooLittleSpeech,
}

/// Enrols, lists and deletes voiceprints.
pub struct VoiceprintService {
    deps: VoiceprintDeps,
}

impl VoiceprintService {
    /// Creates the service over its ports.
    pub fn new(deps: VoiceprintDeps) -> Self {
        Self { deps }
    }

    /// Saves `speaker`'s voice in `meeting` as an exemplar of `person`,
    /// keeping their best [`MAX_VOICEPRINTS_PER_PERSON`].
    pub async fn enrol(
        &self,
        meeting: &MeetingId,
        speaker: SpeakerId,
        person: PersonId,
    ) -> Result<Enrolment, VoiceprintError> {
        let voice = self.deps.speakers.voice(&speaker).await?;
        let Some(embedding) =
            voice.and_then(|v| VoiceEmbedding::new(v.centroid, v.embedding_model).ok())
        else {
            return Ok(Enrolment::Skipped(EnrolSkip::NoVoice));
        };
        let segments = self.deps.segments.list(meeting, Page::ALL).await?;
        let clean = clean_speech(&segments, speaker);
        if clean < MIN_ENROL_SPEECH {
            return Ok(Enrolment::Skipped(EnrolSkip::TooLittleSpeech));
        }
        let print = Voiceprint {
            id: VoiceprintId::generate(self.deps.clock.as_ref()),
            person_id: person,
            embedding,
            source_meeting_id: Some(meeting.clone()),
            quality: quality_of(clean),
            created_at: self.deps.clock.now(),
        };
        let evicted = self
            .deps
            .repo
            .enrol(&print, MAX_VOICEPRINTS_PER_PERSON)
            .await?;
        log::info!(
            target: "deino_core::voiceprints",
            "enrolled voiceprint {} for person {person} from meeting {meeting}; {} evicted",
            print.id, evicted.len()
        );
        Ok(Enrolment::Enrolled {
            id: print.id,
            evicted: evicted.len(),
        })
    }

    /// Every person with a voice profile, by name, best exemplar first.
    pub async fn profiles(&self) -> Result<Vec<VoiceProfile>, VoiceprintError> {
        Ok(group_profiles(self.deps.repo.profile_rows().await?))
    }

    /// Deletes one exemplar.
    pub async fn delete(&self, id: &VoiceprintId) -> Result<(), VoiceprintError> {
        if self.deps.repo.delete(id).await? {
            Ok(())
        } else {
            Err(VoiceprintError::NotFound(*id))
        }
    }

    /// Deletes a person's whole voice profile; how many exemplars it had.
    pub async fn forget(&self, person: &PersonId) -> Result<u64, VoiceprintError> {
        Ok(self.deps.repo.forget(person).await?)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
