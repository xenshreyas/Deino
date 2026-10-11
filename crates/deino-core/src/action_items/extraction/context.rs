//! [`ContextSources::load`]: everything extraction needs to know about a
//! meeting, gathered once: when it happened (in the user's time zone), who
//! was there, what the user noted, and the numbered transcript, plus the
//! [`Roster`] that names are resolved against.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use deino_db::DbError;
use deino_domain::{Channel, LocalZone, MeetingId, SpeakerId};
use deino_llm::extraction::{ME_SPEAKER, attendee_line, meeting_header};

use super::assignee::{Candidate, Roster};
use super::evidence::{Transcript, TranscriptLine};
use super::llm::ExtractionPrompt;
use crate::meetings::{MeetingLifecycleRepo, MeetingRepo};
use crate::notes::NotesRepo;
use crate::people::{AttendeeRepo, MeRepo, Person, PersonRepo};
use crate::segments::{Page, Segment, SegmentReader, Speaker, SpeakerRepo};

/// The most of the user's notes a prompt carries, in characters.
pub const MAX_NOTES_CHARS: usize = 8_000;

/// Where the meeting's context is read from.
#[derive(Clone)]
pub struct ContextSources {
    /// Titles and save times.
    pub meetings: Arc<dyn MeetingRepo>,
    /// Start times.
    pub lifecycle: Arc<dyn MeetingLifecycleRepo>,
    /// The transcript.
    pub segments: Arc<dyn SegmentReader>,
    /// Who each voice is.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// Who was invited.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// Names and titles.
    pub people: Arc<dyn PersonRepo>,
    /// Who "Me" is.
    pub me: Arc<dyn MeRepo>,
    /// The user's notes (NOT-01's notepad), read as Markdown.
    pub notes: Arc<dyn NotesRepo>,
}

/// One meeting, ready to extract from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingContext {
    /// The user's local day the meeting happened on; due phrases count from it.
    pub day: NaiveDate,
    /// The prompt's text.
    pub prompt: ExtractionPrompt,
    /// The numbered transcript, for checking citations.
    pub transcript: Transcript,
    /// The people names resolve to.
    pub roster: Roster,
}

impl ContextSources {
    /// The meeting's context; `None` when there is no such meeting.
    pub async fn load(
        &self,
        meeting: &MeetingId,
        zone: &dyn LocalZone,
    ) -> Result<Option<MeetingContext>, DbError> {
        let Some(saved) = self.meetings.find(meeting).await? else {
            return Ok(None);
        };
        let started = self
            .lifecycle
            .state(meeting)
            .await?
            .and_then(|state| state.started_at);
        let held = started.unwrap_or(saved.created_at);
        let me = self.me.me().await?;
        let attendees = self.attendee_people(meeting).await?;
        let speakers = self.speaker_people(meeting).await?;
        let transcript = self.transcript(meeting, &speakers).await?;
        let notes = self.notes(meeting).await?;
        let prompt = ExtractionPrompt {
            meeting: meeting_header(&saved.title, zone.local(held)),
            attendees: attendee_lines(&attendees, me.as_ref()),
            notes,
            transcript: transcript.render(),
        };
        Ok(Some(MeetingContext {
            day: zone.day_of(held),
            prompt,
            transcript,
            roster: roster(me.as_ref(), &attendees, &speakers),
        }))
    }

    /// The user's notes as Markdown, clipped; `None` when they wrote none.
    async fn notes(&self, meeting: &MeetingId) -> Result<Option<String>, DbError> {
        let stored = self.notes.get(meeting).await?;
        Ok(stored
            .map(|notes| {
                notes
                    .markdown
                    .chars()
                    .take(MAX_NOTES_CHARS)
                    .collect::<String>()
            })
            .filter(|markdown| !markdown.trim().is_empty()))
    }

    async fn attendee_people(&self, meeting: &MeetingId) -> Result<Vec<Person>, DbError> {
        let mut people = Vec::new();
        for attendee in self.attendees.attendees(meeting).await? {
            people.extend(self.people.find(&attendee.person_id).await?);
        }
        Ok(people)
    }

    /// Each speaker with the person it was identified as.
    async fn speaker_people(
        &self,
        meeting: &MeetingId,
    ) -> Result<Vec<(Speaker, Option<Person>)>, DbError> {
        let mut speakers = Vec::new();
        for speaker in self.speakers.speakers(meeting).await? {
            let person = match speaker.person_id {
                Some(id) => self.people.find(&id).await?,
                None => None,
            };
            speakers.push((speaker, person));
        }
        Ok(speakers)
    }

    async fn transcript(
        &self,
        meeting: &MeetingId,
        speakers: &[(Speaker, Option<Person>)],
    ) -> Result<Transcript, DbError> {
        let names: HashMap<SpeakerId, String> = speakers
            .iter()
            .map(|(speaker, person)| (speaker.id, speaker_name(speaker, person.as_ref())))
            .collect();
        let segments = self.segments.list(meeting, Page::ALL).await?;
        let lines = segments
            .into_iter()
            .filter(|segment| !segment.is_deleted)
            .map(|segment| line(segment, &names))
            .collect();
        Ok(Transcript { lines })
    }
}

/// One `- Name (me), Title` line per attendee.
fn attendee_lines(attendees: &[Person], me: Option<&Person>) -> String {
    attendees
        .iter()
        .map(|person| {
            let is_me = Some(person.id) == me.map(|me| me.id);
            attendee_line(&person.name, is_me, person.title.as_deref())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The name a speaker goes by in the prompt.
fn speaker_name(speaker: &Speaker, person: Option<&Person>) -> String {
    match person {
        _ if speaker.is_me => ME_SPEAKER.to_owned(),
        Some(person) => person.name.clone(),
        None => speaker.label.clone(),
    }
}

fn line(segment: Segment, names: &HashMap<SpeakerId, String>) -> TranscriptLine {
    let speaker = segment
        .speaker_id
        .and_then(|id| names.get(&id).cloned())
        .unwrap_or_else(|| channel_name(segment.channel).to_owned());
    TranscriptLine {
        segment: segment.id,
        start: segment.start,
        speaker,
        text: segment.text,
    }
}

/// The voice of an unattributed segment: the microphone is the user.
fn channel_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Mic => ME_SPEAKER,
        Channel::System => "Them",
        Channel::Mixed => "Speaker",
    }
}

/// Attendees first, then the people (and labels) of identified speakers.
fn roster(
    me: Option<&Person>,
    attendees: &[Person],
    speakers: &[(Speaker, Option<Person>)],
) -> Roster {
    let candidate = |person: &Person, name: &str| Candidate {
        person: person.id,
        name: name.to_owned(),
        email: person.email.as_ref().map(|email| email.as_str().to_owned()),
    };
    let mut voices = Vec::new();
    for (speaker, person) in speakers {
        if let Some(person) = person {
            voices.push(candidate(person, &person.name));
            voices.push(candidate(person, &speaker.label));
        }
    }
    Roster {
        me: me.map(|me| me.id),
        attendees: attendees
            .iter()
            .map(|person| candidate(person, &person.name))
            .collect(),
        speakers: voices,
    }
}
