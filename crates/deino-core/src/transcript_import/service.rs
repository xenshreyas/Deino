//! [`TranscriptImporter`]: writes an imported transcript as an ordinary,
//! finished meeting through the same repositories the sample importer uses.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use deino_doc::Flavor;
use deino_domain::{
    Channel, Clock, MeetingId, MeetingSource, MeetingStatus, Millis, PaletteColor, Seconds,
    SegmentId, SpeakerId, TagSource,
};

use super::model::Line;
use super::{MAX_UTTERANCES, TranscriptImport, TranscriptImportError};
use crate::enhance::{EnhancedNotesWriter, NewVersion, ai_doc};
use crate::meetings::{
    MeetingLifecycleRepo, MeetingTiming, MeetingTitle, MeetingTrashRepo, MeetingWork, NewMeeting,
};
use crate::segments::{NewSegment, SegmentWriter, Speaker, SpeakerRepo, WordTiming};
use crate::tags::{NewTag, TagService};

/// The `model` recorded on imported notes (no LLM wrote them).
const IMPORT_MODEL: &str = "deino:api-import";
/// How long a word takes, for the end of the last timed utterance.
const MS_PER_WORD: u32 = 400;

/// What the importer writes through.
pub struct TranscriptImportPorts {
    /// Creates the meeting and records its timing.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// Removes a half-written meeting.
    pub trash: Arc<dyn MeetingTrashRepo>,
    /// The transcript's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The transcript's segments.
    pub segments: Arc<dyn SegmentWriter>,
    /// The notes.
    pub notes: Arc<dyn EnhancedNotesWriter>,
    /// Finds, makes and applies tags.
    pub tags: Arc<TagService>,
    /// Indexes the new meeting for search.
    pub work: Arc<dyn MeetingWork>,
    /// Stamps the import.
    pub clock: Arc<dyn Clock>,
}

/// Imports transcripts sent as JSON.
pub struct TranscriptImporter {
    ports: TranscriptImportPorts,
}

impl TranscriptImporter {
    /// The importer over `ports`.
    pub fn new(ports: TranscriptImportPorts) -> Self {
        Self { ports }
    }

    /// Writes `import` as a new `ready` meeting and returns its ID. A
    /// failed import removes what it wrote.
    pub async fn import(&self, import: &TranscriptImport) -> Result<MeetingId, TranscriptImportError> {
        let title = MeetingTitle::parse(&import.name)?;
        let lines = lines(import)?;
        let now = self.ports.clock.now();
        let id = MeetingId::new(format!(
            "meeting-{}",
            SegmentId::generate(self.ports.clock.as_ref())
        ));
        let at = import.created_at.unwrap_or(now);
        if let Err(error) = self.write(&id, (title, at), import, &lines).await {
            if let Err(cleanup) = self.ports.trash.delete(&id).await {
                log::warn!(target: "api", "could not remove a half-imported meeting: {cleanup}");
            }
            return Err(error);
        }
        self.ports.work.reindex(&id).await;
        log::info!(target: "api", "imported meeting {id} ({} segments)", lines.len());
        Ok(id)
    }

    async fn write(
        &self,
        id: &MeetingId,
        (title, at): (MeetingTitle, DateTime<Utc>),
        import: &TranscriptImport,
        lines: &[Line],
    ) -> Result<(), TranscriptImportError> {
        self.create_meeting(id, title, at, lines).await?;
        let speakers = self.add_speakers(id, lines).await?;
        self.add_segments(id, lines, &speakers).await?;
        if let Some(doc) = import.notes.as_deref().and_then(ai_doc) {
            let version = NewVersion {
                meeting_id: id.clone(),
                markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
                doc,
                template_id: None,
                model: IMPORT_MODEL.to_owned(),
                basis: None,
                is_user_edited: false,
                at: self.ports.clock.now(),
            };
            self.ports.notes.insert(&version).await?;
        }
        self.apply_tags(id, import).await
    }

    async fn create_meeting(
        &self,
        id: &MeetingId,
        title: MeetingTitle,
        at: DateTime<Utc>,
        lines: &[Line],
    ) -> Result<(), TranscriptImportError> {
        let length = lines.last().map_or(0, |line| end_of(line, None).get());
        let meeting = NewMeeting {
            id: id.clone(),
            title,
            status: MeetingStatus::Ready,
            source: MeetingSource::Import,
            created_at: at,
            started_at: Some(at),
            folder_path: None,
        };
        self.ports.meetings.create(&meeting).await?;
        let timing = MeetingTiming {
            started_at: Some(at),
            ended_at: Some(at + TimeDelta::milliseconds(i64::from(length))),
            duration: Some(Seconds(length / 1_000)),
        };
        self.ports.meetings.set_timing(id, &timing, at).await?;
        Ok(())
    }

    /// One speaker per name, in order of first appearance.
    async fn add_speakers(
        &self,
        id: &MeetingId,
        lines: &[Line],
    ) -> Result<HashMap<String, SpeakerId>, TranscriptImportError> {
        let mut ids = HashMap::new();
        for line in lines {
            if ids.contains_key(&line.speaker) {
                continue;
            }
            let slot = u8::try_from(ids.len() % usize::from(PaletteColor::SLOTS)).unwrap_or(0);
            let speaker = Speaker {
                id: SpeakerId::generate(self.ports.clock.as_ref()),
                meeting_id: id.clone(),
                label: line.speaker.clone(),
                person_id: None,
                is_me: line.speaker.eq_ignore_ascii_case("me"),
                color: PaletteColor::new(slot + 1).unwrap_or(PaletteColor::FIRST),
            };
            self.ports.speakers.add_speaker(&speaker).await?;
            ids.insert(line.speaker.clone(), speaker.id);
        }
        Ok(ids)
    }

    async fn add_segments(
        &self,
        id: &MeetingId,
        lines: &[Line],
        speakers: &HashMap<String, SpeakerId>,
    ) -> Result<(), TranscriptImportError> {
        let segments: Vec<NewSegment> = lines
            .iter()
            .enumerate()
            .map(|(index, line)| NewSegment {
                id: SegmentId::generate(self.ports.clock.as_ref()),
                meeting_id: id.clone(),
                channel: Channel::Mixed,
                speaker_id: speakers.get(&line.speaker).copied(),
                start: line.start,
                end: end_of(line, lines.get(index + 1)),
                text: line.text.clone(),
                words: line
                    .words
                    .iter()
                    .map(|(text, start, end)| WordTiming {
                        text: text.clone(),
                        start: *start,
                        end: *end,
                    })
                    .collect(),
            })
            .collect();
        self.ports.segments.append(&segments).await?;
        Ok(())
    }

    /// Puts each tag on the meeting, making the ones that do not exist yet.
    async fn apply_tags(
        &self,
        id: &MeetingId,
        import: &TranscriptImport,
    ) -> Result<(), TranscriptImportError> {
        let existing = self.ports.tags.list().await?;
        for wanted in &import.tags {
            let found = existing
                .iter()
                .find(|tag| tag.name.eq_ignore_ascii_case(wanted.name.trim()));
            let tag = match found {
                Some(tag) => tag.id,
                None => {
                    let new = NewTag {
                        name: wanted.name.clone(),
                        color: None,
                        description: wanted.description.clone(),
                    };
                    self.ports.tags.create(&new).await?.id
                }
            };
            self.ports.tags.apply(id, &tag, TagSource::User).await?;
        }
        Ok(())
    }
}

/// The import's utterances as lines, checked.
fn lines(import: &TranscriptImport) -> Result<Vec<Line>, TranscriptImportError> {
    if import.transcript.len() > MAX_UTTERANCES {
        return Err(TranscriptImportError::Invalid("the transcript has too many utterances"));
    }
    let mut lines = Vec::with_capacity(import.transcript.len());
    for utterance in &import.transcript {
        let line = utterance.line().map_err(TranscriptImportError::Invalid)?;
        if line.speaker.is_empty() || line.text.is_empty() {
            return Err(TranscriptImportError::Invalid("an utterance has no speaker or text"));
        }
        if lines.last().is_some_and(|last: &Line| line.start < last.start) {
            return Err(TranscriptImportError::Invalid("utterances are not in time order"));
        }
        lines.push(line);
    }
    Ok(lines)
}

/// Where `line` ends: its last word, else where the next line starts, else
/// a guess from its length.
fn end_of(line: &Line, next: Option<&Line>) -> Millis {
    if let Some(end) = line.end {
        return end;
    }
    if let Some(next) = next {
        return next.start.max(line.start);
    }
    let words = u32::try_from(line.text.split_whitespace().count()).unwrap_or(u32::MAX);
    Millis(line.start.get().saturating_add(words.saturating_mul(MS_PER_WORD).max(1_000)))
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
