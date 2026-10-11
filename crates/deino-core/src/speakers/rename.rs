//! Renaming a speaker: "this segment" or "all segments" (CAP-04).
//!
//! - **This segment** moves one segment to the person's speaker in the
//!   meeting, creating one when they have none. It proves nothing about the
//!   voice, so nothing is enrolled.
//! - **All segments** makes the speaker the person. When the person already
//!   has another speaker in the meeting, the two are one voice and merge.
//!   It is the user's confirmation of the voice, so it is the only rename
//!   that enrols a voiceprint (spike: enrolment errors propagate).
//!
//! A typed name creates a minimal person (name only); ORG-04's People pages
//! fill in the rest.

use deino_domain::{MeetingId, PaletteColor, PersonId, SegmentId, SpeakerId};

use super::service::find;
use super::{
    MAX_NAME_CHARS, NameSpeaker, PersonRef, Rename, RenameScope, Renamed, SegmentTarget,
    SpeakerError, SpeakerName, SpeakerService,
};
use crate::diarization::speaker_labels;
use crate::people::Person;
use crate::segments::Speaker;
use crate::voiceprints::Enrolment;

/// The meeting a rename happens in, with its speakers before it.
struct Room<'a> {
    meeting: &'a MeetingId,
    speakers: &'a [Speaker],
}

impl SpeakerService {
    /// Names the speaker as `rename` asks; see the module docs.
    pub async fn rename(&self, rename: &Rename) -> Result<Renamed, SpeakerError> {
        let meeting = &rename.meeting_id;
        let speakers = self.ports.speakers.speakers(meeting).await?;
        let speaker = find(&speakers, rename.speaker_id)?.clone();
        let person = self.resolve(&rename.name).await?;
        let room = Room {
            meeting,
            speakers: &speakers,
        };
        match &rename.scope {
            RenameScope::Segment { segment_id } => {
                self.rename_segment(&room, *segment_id, person).await
            }
            RenameScope::All => self.rename_all(&room, speaker, person).await,
        }
    }

    /// Moves one segment to the person's speaker.
    async fn rename_segment(
        &self,
        room: &Room<'_>,
        segment: SegmentId,
        person: PersonRef,
    ) -> Result<Renamed, SpeakerError> {
        let existing = room
            .speakers
            .iter()
            .find(|s| s.person_id == Some(person.id));
        let target = match existing {
            Some(existing) => SegmentTarget::Existing(existing.id),
            None => SegmentTarget::New(self.new_speaker(room, person.id)),
        };
        let naming = &self.ports.naming;
        if !naming.name_segment(room.meeting, &segment, &target).await? {
            return Err(SpeakerError::SegmentNotFound(segment));
        }
        let speaker_id = match target {
            SegmentTarget::Existing(id) => id,
            SegmentTarget::New(speaker) => speaker.id,
        };
        Ok(Renamed {
            person,
            speaker_id,
            enrolled: false,
        })
    }

    /// Makes the speaker the person everywhere and enrols its voice.
    async fn rename_all(
        &self,
        room: &Room<'_>,
        speaker: Speaker,
        person: PersonRef,
    ) -> Result<Renamed, SpeakerError> {
        let meeting = room.meeting;
        let voiceprints = &self.ports.voiceprints;
        let enrolment = voiceprints.enrol(meeting, speaker.id, person.id).await?;
        let twin = room
            .speakers
            .iter()
            .find(|other| other.id != speaker.id && other.person_id == Some(person.id))
            .cloned();
        let speaker_id = match twin {
            Some(twin) => self.join(meeting, [speaker, twin], person.id).await?,
            None => self.assign(meeting, &speaker, person.id).await?,
        };
        Ok(Renamed {
            person,
            speaker_id,
            enrolled: matches!(enrolment, Enrolment::Enrolled { .. }),
        })
    }

    /// The renamed speaker and the person's other speaker become one, named
    /// `person`: the user's own microphone speaker always survives,
    /// otherwise the one already named does.
    async fn join(
        &self,
        meeting: &MeetingId,
        [renamed, twin]: [Speaker; 2],
        person: PersonId,
    ) -> Result<SpeakerId, SpeakerError> {
        let (from, into) = if renamed.is_me {
            (twin, renamed)
        } else {
            (renamed, twin)
        };
        self.fold(meeting, from, into.clone()).await?;
        if into.person_id != Some(person) {
            self.assign(meeting, &into, person).await?;
        }
        Ok(into.id)
    }

    /// Names one speaker, moving its previous person's action items along.
    async fn assign(
        &self,
        meeting: &MeetingId,
        speaker: &Speaker,
        person: PersonId,
    ) -> Result<SpeakerId, SpeakerError> {
        let change = NameSpeaker {
            meeting_id: meeting.clone(),
            speaker_id: speaker.id,
            previous: speaker.person_id,
            person,
            at: self.ports.clock.now(),
        };
        if !self.ports.naming.name_speaker(&change).await? {
            return Err(SpeakerError::SpeakerNotFound(speaker.id));
        }
        Ok(speaker.id)
    }

    /// The person a name stands for, creating someone new for a typed name.
    async fn resolve(&self, name: &SpeakerName) -> Result<PersonRef, SpeakerError> {
        let person = match name {
            SpeakerName::Person { person_id } => self.existing(*person_id).await?,
            SpeakerName::New { name } => self.create_person(name).await?,
        };
        Ok(PersonRef {
            id: person.id,
            name: person.name,
        })
    }

    /// An existing person; one merged into another stands for that one.
    async fn existing(&self, id: PersonId) -> Result<Person, SpeakerError> {
        let people = &self.ports.people;
        let person = people
            .find(&id)
            .await?
            .ok_or(SpeakerError::PersonNotFound(id))?;
        let Some(target) = person.merged_into else {
            return Ok(person);
        };
        people
            .find(&target)
            .await?
            .ok_or(SpeakerError::PersonNotFound(target))
    }

    /// A minimal new person with only a name.
    async fn create_person(&self, name: &str) -> Result<Person, SpeakerError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
            return Err(SpeakerError::InvalidName);
        }
        let now = self.ports.clock.now();
        let person = Person {
            id: PersonId::generate(self.ports.clock.as_ref()),
            name: name.to_owned(),
            email: None,
            company_id: None,
            title: None,
            avatar_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            is_me: false,
            merged_into: None,
            created_at: now,
            updated_at: now,
        };
        self.ports.people.create(&person).await?;
        Ok(person)
    }

    /// A new "Speaker N" for `person`, in the next palette slot.
    fn new_speaker(&self, room: &Room<'_>, person: PersonId) -> Speaker {
        let count = room.speakers.len();
        let taken: Vec<String> = room.speakers.iter().map(|s| s.label.clone()).collect();
        let label = speaker_labels(1, &taken).into_iter().next();
        Speaker {
            id: SpeakerId::generate(self.ports.clock.as_ref()),
            meeting_id: room.meeting.clone(),
            label: label.unwrap_or_else(|| format!("Speaker {}", count + 1)),
            person_id: Some(person),
            is_me: false,
            color: PaletteColor::cycle(count),
        }
    }
}
