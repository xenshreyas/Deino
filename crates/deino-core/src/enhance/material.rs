//! [`Material`]: everything the notes are written from, as
//! [`MaterialSources`](super::MaterialSources) gathers it.
//!
//! The transcript with speaker names and channels, the user's notes, the
//! calendar title, attendees and description, the previous meeting of the
//! series (its notes only, cut short) and the vocabulary. The profile and
//! the template are not here: the gateway adds the profile to every prompt
//! (NOT-05) and the service picks the template (NOT-04).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_doc::Doc;
use deino_domain::{MeetingId, PersonId, SeriesId};

use super::transcript::TranscriptLine;

/// Terms the transcript should be read with (TRN-05's custom vocabulary).
#[async_trait]
pub trait Vocabulary: Send + Sync {
    /// The terms that apply to `meeting`, most important first.
    async fn terms(&self, meeting: &MeetingId) -> Result<Vec<String>, DbError>;
}

/// No vocabulary yet: TRN-05 replaces it with the user's terms.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoVocabulary;

#[async_trait]
impl Vocabulary for NoVocabulary {
    async fn terms(&self, _meeting: &MeetingId) -> Result<Vec<String>, DbError> {
        Ok(Vec::new())
    }
}

/// Everything the notes are written from.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// The meeting and its people.
    pub meeting: MeetingInfo,
    /// The user's own notes, when they wrote any.
    pub notes: Option<UserNotes>,
    /// The transcript in time order, deleted segments left out.
    pub transcript: Vec<TranscriptLine>,
    /// When a transcript segment was last edited.
    pub transcript_edited_at: Option<DateTime<Utc>>,
    /// Who attended.
    pub attendee_ids: Vec<PersonId>,
    /// The notes of the series' previous meeting.
    pub previous: Option<PreviousNotes>,
    /// Terms to spell as given.
    pub vocabulary: Vec<String>,
}

impl Material {
    /// True when there is nothing to write notes from.
    pub fn is_empty(&self) -> bool {
        self.transcript.is_empty() && self.notes.is_none()
    }
}

/// The meeting as the prompt describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingInfo {
    /// The meeting.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it started, or was created.
    pub date: DateTime<Utc>,
    /// The linked calendar event's title, if any.
    pub calendar_title: Option<String>,
    /// The calendar event's description; empty without one.
    pub description: String,
    /// The attendees.
    pub attendees: Vec<AttendeeInfo>,
    /// Its recurring series.
    pub series_id: Option<SeriesId>,
}

/// One attendee.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendeeInfo {
    /// Their name (or address).
    pub name: String,
    /// Their job title.
    pub title: Option<String>,
    /// True for the user.
    pub is_me: bool,
}

/// The user's notes for the meeting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserNotes {
    /// The document they typed.
    pub doc: Doc,
    /// Its Readable Markdown.
    pub markdown: String,
    /// When they last saved it.
    pub updated_at: DateTime<Utc>,
}

/// The notes of an earlier meeting of the same series.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviousNotes {
    /// Its title.
    pub title: String,
    /// When it happened.
    pub date: DateTime<Utc>,
    /// Its newest notes, as Readable Markdown.
    pub markdown: String,
}
