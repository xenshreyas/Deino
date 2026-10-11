//! [`ExportAssembler`]: one meeting, read from the repositories into the
//! [`MeetingExport`] every renderer works from.
//!
//! Names are resolved here, once: attendees and assignees by person, and
//! transcript speakers by the person they were identified as, "Me" for the
//! user's voice, or their diarization label. Invitees who declined are not
//! attendees. Times are converted to the user's zone.

use std::collections::HashMap;
use std::sync::Arc;

use deino_domain::{Channel, MeetingId, PersonId, Rsvp, SpeakerId};
use deino_export::{ActionItemExport, MeetingExport, Participant, Utterance};

use deino_db::Database;

use super::{DocNoteRenderer, ExportError, LocalZone, NoteRenderer};
use crate::action_items::{ActionItemRepo, SqliteActionItemRepo};
use crate::enhance::SqliteEnhancedNotesRepo;
use crate::folders::{MeetingFolderRepo, SqliteFolderRepo};
use crate::meetings::{MeetingLifecycleRepo, MeetingRepo, SqliteMeetingRepo};
use crate::notes::SqliteNotesRepo;
use crate::people::{AttendeeRepo, PersonRepo, SqlitePeopleRepo};
use crate::segments::{Page, Segment, SegmentReader, Speaker, SpeakerRepo, SqliteSegmentRepo};
use crate::tags::{MeetingTagRepo, SqliteTagRepo};

/// What a speaker without a person is called when it is the user.
const ME: &str = "Me";
/// What the other side of a call is called before diarization names it.
const THEM: &str = "Them";
/// What a voice on mixed audio is called before diarization names it.
const SPEAKER: &str = "Speaker";

/// The repositories an export reads.
#[derive(Clone)]
pub struct ExportPorts {
    /// Titles and creation times.
    pub meetings: Arc<dyn MeetingRepo>,
    /// Start time and duration.
    pub lifecycle: Arc<dyn MeetingLifecycleRepo>,
    /// Who attended.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// Their names.
    pub people: Arc<dyn PersonRepo>,
    /// Tags on the meeting.
    pub tags: Arc<dyn MeetingTagRepo>,
    /// Folders the meeting is in.
    pub folders: Arc<dyn MeetingFolderRepo>,
    /// The meeting's action items.
    pub action_items: Arc<dyn ActionItemRepo>,
    /// The transcript.
    pub segments: Arc<dyn SegmentReader>,
    /// The transcript's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The notes, as Markdown.
    pub notes: Arc<dyn NoteRenderer>,
    /// The user's time zone.
    pub zone: Arc<dyn LocalZone>,
}

impl ExportPorts {
    /// Every port over the SQLite repositories of `database`, the stored
    /// Markdown notes and `zone`.
    pub fn sqlite(database: &Database, zone: Arc<dyn LocalZone>) -> Self {
        let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
        let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        Self {
            meetings: meetings.clone(),
            lifecycle: meetings,
            attendees: people.clone(),
            people,
            tags: Arc::new(SqliteTagRepo::new(database.clone())),
            folders: Arc::new(SqliteFolderRepo::new(database.clone())),
            action_items: Arc::new(SqliteActionItemRepo::new(database.clone())),
            segments: segments.clone(),
            speakers: segments,
            notes: Arc::new(DocNoteRenderer::new(
                Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
                Arc::new(SqliteNotesRepo::new(database.clone())),
            )),
            zone,
        }
    }
}

/// Reads meetings into [`MeetingExport`]s.
pub struct ExportAssembler {
    ports: ExportPorts,
}

impl ExportAssembler {
    /// An assembler over `ports`.
    pub fn new(ports: ExportPorts) -> Self {
        Self { ports }
    }

    /// Every meeting outside the trash, newest first.
    pub async fn meeting_ids(&self) -> Result<Vec<MeetingId>, ExportError> {
        let meetings = self.ports.meetings.list().await?;
        Ok(meetings.into_iter().map(|meeting| meeting.id).collect())
    }

    /// The meeting with `id`, everything an export can include.
    pub async fn assemble(&self, id: &MeetingId) -> Result<MeetingExport, ExportError> {
        let ports = &self.ports;
        let meeting = ports
            .meetings
            .find(id)
            .await?
            .ok_or_else(|| ExportError::NotFound(id.clone()))?;
        let state = ports.lifecycle.state(id).await?;
        let started = state
            .as_ref()
            .and_then(|state| state.started_at)
            .unwrap_or(meeting.created_at);
        let notes = ports.notes.notes(id).await?;
        Ok(MeetingExport {
            id: meeting.id,
            title: meeting.title,
            started_at: ports.zone.local(started),
            duration: state.and_then(|state| state.duration),
            attendees: self.attendees(id).await?,
            tags: ports
                .tags
                .tags_of(id)
                .await?
                .into_iter()
                .map(|applied| applied.tag.name)
                .collect(),
            folders: ports
                .folders
                .folders_of(id)
                .await?
                .into_iter()
                .map(|folder| folder.name)
                .collect(),
            notes: notes.enhanced,
            my_notes: notes.mine,
            action_items: self.action_items(id).await?,
            transcript: self.transcript(id).await?,
        })
    }

    /// Who attended `id`, by name: invitees who declined are left out.
    pub async fn attendees(&self, id: &MeetingId) -> Result<Vec<Participant>, ExportError> {
        let mut participants = Vec::new();
        for attendee in self.ports.attendees.attendees(id).await? {
            if attendee.rsvp == Some(Rsvp::Declined) {
                continue;
            }
            if let Some(person) = self.ports.people.find(&attendee.person_id).await? {
                participants.push(Participant {
                    name: person.name,
                    email: person.email.map(|email| email.as_str().to_owned()),
                    is_me: person.is_me,
                });
            }
        }
        Ok(participants)
    }

    async fn action_items(&self, id: &MeetingId) -> Result<Vec<ActionItemExport>, ExportError> {
        let mut items = Vec::new();
        for item in self.ports.action_items.list_for_meeting(id).await? {
            let assignee = match &item.assignee_person_id {
                Some(person) => self.name_of(person).await?,
                None => None,
            };
            items.push(ActionItemExport {
                title: item.title,
                description: item.description,
                assignee,
                due: item
                    .due_at
                    .map(|due| self.ports.zone.local(due).date_naive()),
                done: item.completed_at.is_some(),
            });
        }
        Ok(items)
    }

    /// The transcript of `id` in time order, each segment with its
    /// speaker's name (the library's `get_transcript` reads it too).
    pub async fn transcript(&self, id: &MeetingId) -> Result<Vec<Utterance>, ExportError> {
        let speakers = self
            .speaker_names(&self.ports.speakers.speakers(id).await?)
            .await?;
        let segments = self.ports.segments.list(id, Page::ALL).await?;
        Ok(segments
            .into_iter()
            .map(|segment| utterance(segment, &speakers))
            .collect())
    }

    /// Each speaker's name and whether it is the user.
    async fn speaker_names(
        &self,
        speakers: &[Speaker],
    ) -> Result<HashMap<SpeakerId, (String, bool)>, ExportError> {
        let mut names = HashMap::with_capacity(speakers.len());
        for speaker in speakers {
            let person = match &speaker.person_id {
                Some(person) => self.name_of(person).await?,
                None => None,
            };
            let name = person.unwrap_or_else(|| {
                if speaker.is_me {
                    ME.to_owned()
                } else {
                    speaker.label.clone()
                }
            });
            names.insert(speaker.id, (name, speaker.is_me));
        }
        Ok(names)
    }

    async fn name_of(&self, person: &PersonId) -> Result<Option<String>, ExportError> {
        Ok(self
            .ports
            .people
            .find(person)
            .await?
            .map(|person| person.name))
    }
}

/// A segment with its speaker's name; unattributed speech is named by its
/// channel (the microphone is the user).
fn utterance(segment: Segment, speakers: &HashMap<SpeakerId, (String, bool)>) -> Utterance {
    let known = segment.speaker_id.as_ref().and_then(|id| speakers.get(id));
    let (speaker, is_me) = match (known, segment.channel) {
        (Some((name, is_me)), _) => (name.clone(), *is_me),
        (None, Channel::Mic) => (ME.to_owned(), true),
        (None, Channel::System) => (THEM.to_owned(), false),
        (None, Channel::Mixed) => (SPEAKER.to_owned(), false),
    };
    Utterance {
        speaker,
        is_me,
        start: segment.start,
        end: segment.end,
        text: segment.text,
    }
}

#[cfg(test)]
#[path = "assemble_tests.rs"]
mod tests;
