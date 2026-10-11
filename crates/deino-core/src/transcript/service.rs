//! The transcript service: one read that the transcript view renders.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId, PersonId};

use super::{Transcript, TranscriptSpeaker};
use crate::people::{Person, PersonRepo};
use crate::segments::{Page, SegmentReader, Speaker, SpeakerRepo};

/// A transcript failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TranscriptError {
    /// The database failed; the source carries the detail.
    #[error("could not load the transcript")]
    Storage(#[from] DbError),
}

impl ErrorClass for TranscriptError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

/// The ports the transcript reads through.
pub struct TranscriptPorts {
    /// The saved segments.
    pub segments: Arc<dyn SegmentReader>,
    /// The meeting's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The people speakers are identified as.
    pub people: Arc<dyn PersonRepo>,
}

/// Reads meeting transcripts for the UI.
pub struct TranscriptService {
    ports: TranscriptPorts,
}

impl TranscriptService {
    /// Creates the service over its ports.
    pub fn new(ports: TranscriptPorts) -> Self {
        Self { ports }
    }

    /// The saved transcript of `meeting`: segments in spoken order and the
    /// speakers with their display names. A meeting without a transcript
    /// (or an unknown one) has no segments and no speakers.
    pub async fn transcript(&self, meeting: &MeetingId) -> Result<Transcript, TranscriptError> {
        let segments = self.ports.segments.list(meeting, Page::ALL).await?;
        let mut speakers = Vec::new();
        for speaker in self.ports.speakers.speakers(meeting).await? {
            speakers.push(self.named(speaker).await?);
        }
        Ok(Transcript { segments, speakers })
    }

    /// `speaker` with the name the view shows: its person's, else its label.
    async fn named(&self, speaker: Speaker) -> Result<TranscriptSpeaker, DbError> {
        let person = match speaker.person_id {
            Some(id) => self.person(&id).await?,
            None => None,
        };
        Ok(TranscriptSpeaker {
            id: speaker.id,
            person_id: person.as_ref().map(|person| person.id),
            name: person.map_or(speaker.label, |person| person.name),
            is_me: speaker.is_me,
            color: speaker.color,
        })
    }

    /// The person `id` names, or the one it was merged into (one hop: a
    /// merge always points at a person that was not merged itself).
    async fn person(&self, id: &PersonId) -> Result<Option<Person>, DbError> {
        let Some(person) = self.ports.people.find(id).await? else {
            return Ok(None);
        };
        let Some(target) = person.merged_into else {
            return Ok(Some(person));
        };
        Ok(Some(
            self.ports.people.find(&target).await?.unwrap_or(person),
        ))
    }
}
