//! [`SampleImporter`]: writes the sample meeting through the repositories.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use deino_doc::Flavor;
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemStatus, Clock, MeetingId, MeetingSource,
    MeetingStatus, Seconds, SegmentId, SpeakerId,
};

use super::{SampleError, SampleMeeting};
use crate::action_items::{ActionItem, ActionItemRepo};
use crate::enhance::{EnhancedNotesWriter, NewVersion, ai_doc};
use crate::meetings::{
    MeetingLifecycleRepo, MeetingTiming, MeetingTitle, MeetingTrashRepo, NewMeeting,
};
use crate::segments::{NewSegment, SegmentWriter, Speaker, SpeakerRepo};

/// The sample's fixed ID: what marks a meeting as the sample, and what keeps
/// a second import from making a copy.
pub const SAMPLE_MEETING_ID: &str = "meeting-sample";

/// The `model` recorded on the sample's enhanced notes (no LLM wrote them).
const SAMPLE_MODEL: &str = "deino:sample";

/// The repositories and clock the importer writes through.
pub struct SamplePorts {
    /// Creates the meeting and records its timing.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// Restores a trashed sample, and removes a half-written one.
    pub trash: Arc<dyn MeetingTrashRepo>,
    /// The transcript's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The transcript's segments.
    pub segments: Arc<dyn SegmentWriter>,
    /// The enhanced notes.
    pub notes: Arc<dyn EnhancedNotesWriter>,
    /// The action items.
    pub action_items: Arc<dyn ActionItemRepo>,
    /// Stamps the meeting as recorded just now.
    pub clock: Arc<dyn Clock>,
}

/// Imports the bundled [`SampleMeeting`] as an ordinary meeting.
pub struct SampleImporter {
    ports: SamplePorts,
}

impl SampleImporter {
    /// An importer over `ports`.
    pub fn new(ports: SamplePorts) -> Self {
        Self { ports }
    }

    /// The sample meeting's ID, importing it first unless it is already
    /// there (a trashed sample is restored). A failed import removes what it
    /// wrote, so the next attempt starts clean.
    pub async fn import(&self) -> Result<MeetingId, SampleError> {
        let id = MeetingId::new(SAMPLE_MEETING_ID);
        let now = self.ports.clock.now();
        if let Some(state) = self.ports.meetings.state(&id).await? {
            if state.deleted_at.is_some() {
                self.ports.trash.restore(&id, now).await?;
            }
            return Ok(id);
        }
        let sample = SampleMeeting::bundled()?;
        if let Err(error) = self.write(&id, &sample, now).await {
            if let Err(cleanup) = self.ports.trash.delete(&id).await {
                log::warn!(target: "sample", "could not remove a half-written sample: {cleanup}");
            }
            return Err(error);
        }
        log::info!(target: "sample", "imported the sample meeting ({} segments)", sample.segments.len());
        Ok(id)
    }

    async fn write(
        &self,
        id: &MeetingId,
        sample: &SampleMeeting,
        now: DateTime<Utc>,
    ) -> Result<(), SampleError> {
        self.create_meeting(id, sample, now).await?;
        let speakers = self.add_speakers(id, sample).await?;
        let segments = self.add_segments(id, sample, &speakers).await?;
        self.add_notes(id, sample, now).await?;
        self.add_action_items((id, now), sample, &segments).await
    }

    /// A finished meeting that ended now, with no recording files.
    async fn create_meeting(
        &self,
        id: &MeetingId,
        sample: &SampleMeeting,
        now: DateTime<Utc>,
    ) -> Result<(), SampleError> {
        let title = MeetingTitle::parse(&sample.title)
            .map_err(|_| SampleError::Invalid("the title is not a valid meeting title"))?;
        let started_at = now - TimeDelta::milliseconds(i64::from(sample.duration_ms.get()));
        let meeting = NewMeeting {
            id: id.clone(),
            title,
            status: MeetingStatus::Ready,
            source: MeetingSource::Import,
            created_at: now,
            started_at: Some(started_at),
            folder_path: None,
        };
        self.ports.meetings.create(&meeting).await?;
        let timing = MeetingTiming {
            started_at: Some(started_at),
            ended_at: Some(now),
            duration: Some(Seconds(sample.duration_ms.get() / 1_000)),
        };
        self.ports.meetings.set_timing(id, &timing, now).await?;
        Ok(())
    }

    /// One speaker per label; returns each label's ID.
    async fn add_speakers(
        &self,
        id: &MeetingId,
        sample: &SampleMeeting,
    ) -> Result<HashMap<String, SpeakerId>, SampleError> {
        let mut ids = HashMap::new();
        for voice in &sample.speakers {
            let speaker = Speaker {
                id: SpeakerId::generate(self.ports.clock.as_ref()),
                meeting_id: id.clone(),
                label: voice.label.clone(),
                person_id: None,
                is_me: voice.is_me,
                color: voice.color,
            };
            self.ports.speakers.add_speaker(&speaker).await?;
            ids.insert(voice.label.clone(), speaker.id);
        }
        Ok(ids)
    }

    /// The transcript, in one write; returns the segments' IDs in order.
    async fn add_segments(
        &self,
        id: &MeetingId,
        sample: &SampleMeeting,
        speakers: &HashMap<String, SpeakerId>,
    ) -> Result<Vec<SegmentId>, SampleError> {
        let segments: Vec<NewSegment> = sample
            .segments
            .iter()
            .map(|segment| NewSegment {
                id: SegmentId::generate(self.ports.clock.as_ref()),
                meeting_id: id.clone(),
                channel: segment.channel,
                speaker_id: speakers.get(&segment.speaker).copied(),
                start: segment.start_ms,
                end: segment.end_ms,
                text: segment.text.clone(),
                words: Vec::new(),
            })
            .collect();
        self.ports.segments.append(&segments).await?;
        Ok(segments.into_iter().map(|segment| segment.id).collect())
    }

    /// The notes as version 1, marked as AI-written.
    async fn add_notes(
        &self,
        id: &MeetingId,
        sample: &SampleMeeting,
        now: DateTime<Utc>,
    ) -> Result<(), SampleError> {
        let doc = ai_doc(&sample.notes).ok_or(SampleError::Invalid("the notes do not parse"))?;
        let version = NewVersion {
            meeting_id: id.clone(),
            markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
            doc,
            template_id: None,
            model: SAMPLE_MODEL.to_owned(),
            basis: None,
            is_user_edited: false,
            at: now,
        };
        self.ports.notes.insert(&version).await?;
        Ok(())
    }

    /// Open, unassigned items, each linked to the moment it was said.
    async fn add_action_items(
        &self,
        (id, now): (&MeetingId, DateTime<Utc>),
        sample: &SampleMeeting,
        segments: &[SegmentId],
    ) -> Result<(), SampleError> {
        for item in &sample.action_items {
            let action = ActionItem {
                id: ActionItemId::generate(self.ports.clock.as_ref()),
                meeting_id: Some(id.clone()),
                title: item.title.clone(),
                description: item.description.clone(),
                assignee_person_id: None,
                due_at: None,
                status: ActionItemStatus::Open,
                priority: None,
                source_segment_id: segments.get(item.segment).copied(),
                completed_at: None,
                created_by: ActionItemOrigin::Ai,
                created_at: now,
                updated_at: now,
            };
            self.ports.action_items.create(&action).await?;
        }
        Ok(())
    }
}
