//! `SpeakerService`: the participants panel's reads and the user's naming
//! (CAP-04).
//!
//! Renaming lives in [`rename`](super::rename); this file holds the panel's
//! list, the name picker's candidates, merging and suggestions. Names
//! propagate by reference: the transcript, notes input and action items
//! read a speaker's person, so naming a speaker renames it everywhere.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId, Millis, Rsvp, SpeakerId};

use super::{
    MergeSpeakers, NameCandidate, NamingRepo, Participant, ParticipantReader, ParticipantRow,
    PersonRef, Rename, RenameScope, Renamed, SpeakerError, SpeakerName,
};
use crate::people::{AttendeeRepo, Person, PersonRepo};
use crate::segments::{Speaker, SpeakerRepo};
use crate::voiceprints::VoiceprintService;

/// How many names the picker offers.
pub const CANDIDATE_LIMIT: usize = 8;

/// The ports [`SpeakerService`] is built from.
#[derive(Clone)]
pub struct SpeakerPorts {
    /// The panel's rows.
    pub participants: Arc<dyn ParticipantReader>,
    /// Naming writes.
    pub naming: Arc<dyn NamingRepo>,
    /// The meeting's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// People to name speakers after, and new ones.
    pub people: Arc<dyn PersonRepo>,
    /// Who was invited.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// Enrols a confirmed voice.
    pub voiceprints: Arc<VoiceprintService>,
    /// IDs and timestamps.
    pub clock: Arc<dyn Clock>,
}

/// Lists, names and merges a meeting's speakers.
pub struct SpeakerService {
    pub(super) ports: SpeakerPorts,
}

impl SpeakerService {
    /// Creates the service over its ports.
    pub fn new(ports: SpeakerPorts) -> Self {
        Self { ports }
    }

    /// The meeting's speakers with their names, speaking time and share,
    /// and any suggestion; most speech first.
    pub async fn participants(
        &self,
        meeting: &MeetingId,
    ) -> Result<Vec<Participant>, SpeakerError> {
        let rows = self.ports.participants.participants(meeting).await?;
        let total: u64 = rows.iter().map(|row| u64::from(row.speech.get())).sum();
        Ok(rows
            .into_iter()
            .map(|row| participant(row, total))
            .collect())
    }

    /// Up to [`CANDIDATE_LIMIT`] people to name a speaker after: the
    /// meeting's attendees whose name or email starts with `query`, then
    /// everyone else who matches.
    pub async fn candidates(
        &self,
        meeting: &MeetingId,
        query: &str,
    ) -> Result<Vec<NameCandidate>, SpeakerError> {
        let query = query.trim().to_lowercase();
        let mut found: Vec<NameCandidate> = Vec::new();
        for attendee in self.ports.attendees.attendees(meeting).await? {
            if attendee.rsvp == Some(Rsvp::Declined) {
                continue;
            }
            let person = self.ports.people.find(&attendee.person_id).await?;
            if let Some(person) = person.filter(|person| matches(person, &query)) {
                found.push(candidate(person, true));
            }
        }
        let limit = u32::try_from(CANDIDATE_LIMIT).unwrap_or(u32::MAX);
        for person in self.ports.people.search(&query, limit).await? {
            if !found.iter().any(|known| known.person.id == person.id) {
                found.push(candidate(person, false));
            }
        }
        found.truncate(CANDIDATE_LIMIT);
        Ok(found)
    }

    /// Folds `from` into `into`: one voice heard as two speakers.
    pub async fn merge(
        &self,
        meeting: &MeetingId,
        from: SpeakerId,
        into: SpeakerId,
    ) -> Result<(), SpeakerError> {
        if from == into {
            return Err(SpeakerError::SameSpeaker);
        }
        let speakers = self.ports.speakers.speakers(meeting).await?;
        let (from, into) = (find(&speakers, from)?, find(&speakers, into)?);
        if from.is_me {
            return Err(SpeakerError::MergingMe);
        }
        self.fold(meeting, from.clone(), into.clone()).await
    }

    /// "Yes, this is Alex": names the speaker for all its segments after
    /// its suggestion, which enrols the voice.
    pub async fn confirm_suggestion(
        &self,
        meeting: &MeetingId,
        speaker: SpeakerId,
    ) -> Result<Renamed, SpeakerError> {
        let rows = self.ports.participants.participants(meeting).await?;
        let suggestion = rows
            .into_iter()
            .find(|row| row.speaker.id == speaker)
            .and_then(|row| row.suggestion)
            .ok_or(SpeakerError::NoSuggestion(speaker))?;
        let rename = Rename {
            meeting_id: meeting.clone(),
            speaker_id: speaker,
            name: SpeakerName::Person {
                person_id: suggestion.person.id,
            },
            scope: RenameScope::All,
        };
        self.rename(&rename).await
    }

    /// "No": forgets the speaker's suggestion.
    pub async fn dismiss_suggestion(&self, speaker: SpeakerId) -> Result<(), SpeakerError> {
        if self.ports.naming.dismiss_suggestion(&speaker).await? {
            Ok(())
        } else {
            Err(SpeakerError::NoSuggestion(speaker))
        }
    }

    /// Merges two known speakers of the meeting.
    pub(super) async fn fold(
        &self,
        meeting: &MeetingId,
        from: Speaker,
        into: Speaker,
    ) -> Result<(), SpeakerError> {
        let change = MergeSpeakers {
            meeting_id: meeting.clone(),
            from,
            into,
            at: self.ports.clock.now(),
        };
        Ok(self.ports.naming.merge(&change).await?)
    }
}

/// The speaker with `id` among the meeting's.
pub(super) fn find(speakers: &[Speaker], id: SpeakerId) -> Result<&Speaker, SpeakerError> {
    speakers
        .iter()
        .find(|speaker| speaker.id == id)
        .ok_or(SpeakerError::SpeakerNotFound(id))
}

/// A row with its share of `total` speech.
fn participant(row: ParticipantRow, total: u64) -> Participant {
    Participant {
        speaker_id: row.speaker.id,
        label: row.speaker.label,
        color: row.speaker.color,
        is_me: row.speaker.is_me,
        person: row.person,
        share_percent: share_percent(row.speech, total),
        speech: row.speech,
        suggestion: row.suggestion,
    }
}

/// `speech` as a rounded percentage of `total` (0 when nothing was said).
pub fn share_percent(speech: Millis, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    let percent = (u64::from(speech.get()) * 100 + total / 2) / total;
    u8::try_from(percent.min(100)).unwrap_or(100)
}

/// Whether `person`'s name, a word of it, or email starts with `prefix`
/// (already lower case); everyone matches an empty prefix.
fn matches(person: &Person, prefix: &str) -> bool {
    let name = person.name.to_lowercase();
    let email = person.email.as_ref().map(|email| email.as_str().to_owned());
    prefix.is_empty()
        || name.split_whitespace().any(|word| word.starts_with(prefix))
        || name.starts_with(prefix)
        || email.is_some_and(|email| email.starts_with(prefix))
}

fn candidate(person: Person, is_attendee: bool) -> NameCandidate {
    NameCandidate {
        is_me: person.is_me,
        person: PersonRef {
            id: person.id,
            name: person.name,
        },
        is_attendee,
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
