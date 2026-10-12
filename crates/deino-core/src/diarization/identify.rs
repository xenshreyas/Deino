//! `identify_speakers` (order 35, CAP-04): names a diarized meeting's
//! speakers from voice profiles and the calendar.
//!
//! [`IdentifyService::identify`] reads the meeting ([`lineup`]), applies the
//! rules in [`decide`] and writes the result in one transaction
//! ([`IdentificationWriter::apply`]):
//!
//! - the user's microphone speaker is the "Me" person (`people.is_me`);
//! - **speaker tags first** (CAP-05): the names the meeting app showed for
//!   its active speaker ([`TagPriors`], `crate::speaker_tags`) seat the
//!   system track's segments and name or label their speakers; those
//!   speakers are then out of voice matching, so a name from the window
//!   always overrides a voiceprint;
//! - each still unnamed voice is scored against every candidate's exemplars
//!   (the max over up to five), then named by voice, by elimination, or
//!   suggested ("Is this Alex?") for the user to confirm.
//!
//! Speakers the user named are never renamed. The thresholds live in
//! `Settings::speakers` ([`IdentifyPolicy`]), which can switch the stage off.

mod decide;
mod lineup;

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId, PersonId, SpeakerId};

pub use decide::{Ballot, Decisions, MIN_IDENTIFIABLE_SPEECH, NamedBy, Naming, Roster, decide};
pub use lineup::{IdentifyPorts, Lineup};

use crate::settings::{IdentifyThresholds, SettingsService};
use crate::speaker_tags::{NoSpeakerTags, TagPlan, TagPriors};
use crate::speakers::{Identification, IdentificationWriter, SuggestedName};

/// Whether to identify speakers, and with which thresholds
/// (`Settings::speakers`).
#[async_trait]
pub trait IdentifyPolicy: Send + Sync {
    /// The thresholds, or `None` when identification is off.
    async fn thresholds(&self) -> Option<IdentifyThresholds>;
}

#[async_trait]
impl IdentifyPolicy for SettingsService {
    async fn thresholds(&self) -> Option<IdentifyThresholds> {
        let speakers = self.get().await.speakers;
        speakers.auto_identify.then_some(speakers.thresholds)
    }
}

/// Names a processed meeting's speakers.
#[async_trait]
pub trait SpeakerIdentifier: Send + Sync {
    /// Identifies `meeting`'s speakers (see the module docs).
    async fn identify(&self, meeting: &MeetingId) -> Result<Identified, IdentifyError>;
}

/// How identifying a meeting went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Identified {
    /// The rules ran.
    Done {
        /// Speakers named (the user's microphone included).
        named: usize,
        /// Speakers left with a suggestion.
        suggested: usize,
    },
    /// Settings turn identification off.
    Disabled,
    /// The meeting has no speakers (not diarized).
    NoSpeakers,
}

/// Why identification failed. Messages carry no content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IdentifyError {
    /// The database failed.
    #[error("could not load or save the meeting's speaker names")]
    Storage(#[from] DbError),
}

impl ErrorClass for IdentifyError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

/// [`SpeakerIdentifier`] over the repositories and Settings.
pub struct IdentifyService {
    ports: IdentifyPorts,
    policy: Arc<dyn IdentifyPolicy>,
    writer: Arc<dyn IdentificationWriter>,
    tags: Arc<dyn TagPriors>,
}

impl IdentifyService {
    /// Creates the service: `ports` to read the meeting, `policy` for the
    /// thresholds, `writer` for the result. No speaker tags until
    /// [`with_speaker_tags`](Self::with_speaker_tags).
    pub fn new(
        ports: IdentifyPorts,
        policy: Arc<dyn IdentifyPolicy>,
        writer: Arc<dyn IdentificationWriter>,
    ) -> Self {
        Self {
            ports,
            policy,
            writer,
            tags: Arc::new(NoSpeakerTags),
        }
    }

    /// The same service, seating segments by the meeting app's names
    /// (CAP-05) before it matches voices.
    #[must_use]
    pub fn with_speaker_tags(mut self, tags: Arc<dyn TagPriors>) -> Self {
        self.tags = tags;
        self
    }
}

#[async_trait]
impl SpeakerIdentifier for IdentifyService {
    async fn identify(&self, meeting: &MeetingId) -> Result<Identified, IdentifyError> {
        let Some(thresholds) = self.policy.thresholds().await else {
            return Ok(Identified::Disabled);
        };
        let speakers = self.ports.speakers.speakers(meeting).await?;
        if speakers.is_empty() {
            return Ok(Identified::NoSpeakers);
        }
        let tags = self.tags.plan(meeting, &speakers).await?;
        let claimed = tags.claimed.clone();
        let speakers = tags.overlay(speakers);
        let lineup = Lineup::read(&self.ports, meeting, speakers, &claimed).await?;
        let decisions = decide(&lineup.ballot, &thresholds);
        let identification = identification(&lineup, &decisions, tags);
        self.writer.apply(meeting, &identification).await?;
        log::info!(
            target: "deino_core::diarization",
            "identified meeting {meeting}: {} named by voice or elimination, {} suggested, me {}",
            decisions.named.len(),
            identification.suggested.len(),
            lineup.me.is_some()
        );
        Ok(Identified::Done {
            named: identification.named.len(),
            suggested: identification.suggested.len(),
        })
    }
}

/// The decisions as rows to write: the user's microphone first, then the
/// speaker tags' names, then the voices'.
fn identification(
    lineup: &Lineup,
    decisions: &Decisions<SpeakerId, PersonId>,
    tags: TagPlan,
) -> Identification {
    let by_voice = decisions
        .named
        .iter()
        .map(|naming| (naming.speaker, naming.person));
    let named = lineup
        .me
        .into_iter()
        .chain(tags.named)
        .chain(by_voice)
        .collect();
    let suggested = decisions
        .suggested
        .iter()
        .map(|&(speaker_id, person_id, score)| SuggestedName {
            speaker_id,
            person_id,
            score,
        })
        .collect();
    Identification {
        named,
        suggested,
        seating: tags.seating,
    }
}

#[cfg(test)]
#[path = "identify_tests.rs"]
mod tests;
