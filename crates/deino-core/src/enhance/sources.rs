//! [`MaterialSources`]: gathers the [`Material`] through the existing
//! repositories: titles and timing, the calendar event, attendees, the
//! transcript with each speaker named, the user's notes, the series'
//! previous notes and the vocabulary.

use std::collections::HashMap;
use std::sync::Arc;

use deino_db::{Database, DbError};
use deino_domain::{Channel, MeetingId, PersonId, SpeakerId};

use super::material::{
    AttendeeInfo, Material, MeetingInfo, NoVocabulary, PreviousNotes, UserNotes, Vocabulary,
};
use super::transcript::TranscriptLine;
use super::{EnhancedNotesReader, SqliteEnhancedNotesRepo};
use crate::calendar::{CalendarEvent, CalendarEventRepo, SqliteCalendarRepo};
use crate::meetings::{MeetingLifecycleRepo, MeetingRepo, MeetingState, SqliteMeetingRepo};
use crate::notes::{NotesRepo, SqliteNotesRepo};
use crate::people::{AttendeeRepo, MeRepo, Person, PersonRepo, SqlitePeopleRepo};
use crate::segments::{Page, Segment, SegmentReader, SpeakerRepo, SqliteSegmentRepo};

/// What a microphone segment's speaker is called when the user has no name.
const ME: &str = "Me";
/// The speaker of an unattributed system-audio segment.
const THEM: &str = "Them";
/// The speaker of an unattributed mixed segment.
const SOMEONE: &str = "Speaker";

/// The repositories the material is read from.
#[derive(Clone)]
pub struct MaterialSources {
    /// Titles.
    pub meetings: Arc<dyn MeetingRepo>,
    /// Timing, calendar link and series.
    pub lifecycle: Arc<dyn MeetingLifecycleRepo>,
    /// The transcript.
    pub segments: Arc<dyn SegmentReader>,
    /// The transcript's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// People, for attendee and speaker names.
    pub people: Arc<dyn PersonRepo>,
    /// The user's own person.
    pub me: Arc<dyn MeRepo>,
    /// Attendance.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// The linked calendar event.
    pub calendar: Arc<dyn CalendarEventRepo>,
    /// The user's notes.
    pub notes: Arc<dyn NotesRepo>,
    /// Earlier notes of the series.
    pub history: Arc<dyn EnhancedNotesReader>,
    /// Custom vocabulary.
    pub vocabulary: Arc<dyn Vocabulary>,
}

impl MaterialSources {
    /// Every source over Deino's SQLite database, without a vocabulary
    /// (TRN-05 adds one).
    pub fn sqlite(database: &Database) -> Self {
        let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
        Self {
            meetings: meetings.clone(),
            lifecycle: meetings,
            segments: segments.clone(),
            speakers: segments,
            people: people.clone(),
            me: people.clone(),
            attendees: people,
            calendar: Arc::new(SqliteCalendarRepo::new(database.clone())),
            notes: Arc::new(SqliteNotesRepo::new(database.clone())),
            history: Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
            vocabulary: Arc::new(NoVocabulary),
        }
    }

    /// The material of `meeting`, or `None` when it does not exist.
    pub async fn gather(&self, meeting: &MeetingId) -> Result<Option<Material>, DbError> {
        let (Some(found), Some(state)) = (
            self.meetings.find(meeting).await?,
            self.lifecycle.state(meeting).await?,
        ) else {
            return Ok(None);
        };
        let event = self.event(&state).await?;
        let (attendee_ids, attendees) = self.attendees(meeting, event.as_ref()).await?;
        let segments = self.segments.list(meeting, Page::ALL).await?;
        let date = state.started_at.unwrap_or(found.created_at);
        let info = MeetingInfo {
            id: meeting.clone(),
            title: found.title,
            date,
            calendar_title: event.as_ref().map(|event| event.title.clone()),
            description: event.map(|event| event.description).unwrap_or_default(),
            attendees,
            series_id: state.series_id.clone(),
        };
        Ok(Some(Material {
            transcript: self.transcript(meeting, &segments).await?,
            transcript_edited_at: segments.iter().filter_map(|s| s.edited_at).max(),
            notes: self.notes(meeting).await?,
            previous: self.previous(&info).await?,
            vocabulary: self.vocabulary.terms(meeting).await?,
            meeting: info,
            attendee_ids,
        }))
    }

    /// The meeting's transcript lines, speakers named, deleted ones left out.
    pub async fn transcript_of(&self, meeting: &MeetingId) -> Result<Vec<TranscriptLine>, DbError> {
        let segments = self.segments.list(meeting, Page::ALL).await?;
        self.transcript(meeting, &segments).await
    }

    async fn event(&self, state: &MeetingState) -> Result<Option<CalendarEvent>, DbError> {
        match &state.calendar_event_id {
            Some(id) => self.calendar.event(id).await,
            None => Ok(None),
        }
    }

    /// The attendees, from the meeting's people, else the calendar invitation.
    async fn attendees(
        &self,
        meeting: &MeetingId,
        event: Option<&CalendarEvent>,
    ) -> Result<(Vec<PersonId>, Vec<AttendeeInfo>), DbError> {
        let mut ids = Vec::new();
        let mut infos = Vec::new();
        for attendee in self.attendees.attendees(meeting).await? {
            if let Some(person) = self.people.find(&attendee.person_id).await? {
                infos.push(person_info(&person));
                ids.push(person.id);
            }
        }
        if infos.is_empty()
            && let Some(event) = event
        {
            infos = event.attendees.iter().filter_map(invitee_info).collect();
        }
        Ok((ids, infos))
    }

    async fn transcript(
        &self,
        meeting: &MeetingId,
        segments: &[Segment],
    ) -> Result<Vec<TranscriptLine>, DbError> {
        let me = self.me.me().await?;
        let names = self.speaker_names(meeting, me.as_ref()).await?;
        let me = me.map_or_else(|| ME.to_owned(), |me| me.name);
        Ok((1..)
            .zip(segments)
            .map(|(reference, segment)| TranscriptLine {
                reference,
                segment_id: segment.id,
                start: segment.start,
                end: segment.end,
                speaker: speaker_name(segment, &names, &me),
                text: segment.text.clone(),
            })
            .collect())
    }

    /// Each speaker's name: their person's, else the user's, else the label.
    async fn speaker_names(
        &self,
        meeting: &MeetingId,
        me: Option<&Person>,
    ) -> Result<HashMap<SpeakerId, String>, DbError> {
        let mut names = HashMap::new();
        for speaker in self.speakers.speakers(meeting).await? {
            let person = match &speaker.person_id {
                Some(id) => self.people.find(id).await?,
                None if speaker.is_me => me.cloned(),
                None => None,
            };
            let name = person.map_or(speaker.label, |person| person.name);
            names.insert(speaker.id, name);
        }
        Ok(names)
    }

    async fn notes(&self, meeting: &MeetingId) -> Result<Option<UserNotes>, DbError> {
        let notes = self.notes.get(meeting).await?;
        Ok(notes
            .filter(|notes| !notes.doc.is_blank())
            .map(|notes| UserNotes {
                doc: notes.doc,
                markdown: notes.markdown,
                updated_at: notes.updated_at,
            }))
    }

    async fn previous(&self, meeting: &MeetingInfo) -> Result<Option<PreviousNotes>, DbError> {
        match &meeting.series_id {
            Some(series) => {
                self.history
                    .previous_in_series(series, &meeting.id, meeting.date)
                    .await
            }
            None => Ok(None),
        }
    }
}

fn person_info(person: &Person) -> AttendeeInfo {
    AttendeeInfo {
        name: person.name.clone(),
        title: person.title.clone(),
        is_me: person.is_me,
    }
}

fn invitee_info(invitee: &crate::calendar::EventAttendee) -> Option<AttendeeInfo> {
    let name = invitee.name.clone().or_else(|| invitee.email.clone())?;
    Some(AttendeeInfo {
        name,
        title: None,
        is_me: false,
    })
}

/// The segment's speaker by name, else by channel: the user, "Them" or
/// "Speaker".
fn speaker_name(segment: &Segment, names: &HashMap<SpeakerId, String>, me: &str) -> String {
    if let Some(name) = segment.speaker_id.as_ref().and_then(|id| names.get(id)) {
        return name.clone();
    }
    match segment.channel {
        Channel::Mic => me.to_owned(),
        Channel::System => THEM.to_owned(),
        Channel::Mixed => SOMEONE.to_owned(),
    }
}
