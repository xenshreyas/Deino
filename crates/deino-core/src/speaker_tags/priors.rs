//! [`TagPriors`]: what `identify_speakers` asks before matching voices.
//!
//! [`SpeakerTagPriors`] reads the meeting's spans and its system-track
//! segments, finds the name that wins each segment ([`super::align`]),
//! resolves names to people ([`super::resolve`]), seats them
//! ([`super::seats`]) and returns the [`TagPlan`]. A meeting recorded
//! without speaker tags (no permission, no supported app, a window the
//! adapters did not recognise) has no spans, so its plan is empty and voice
//! matching names its speakers exactly as before.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{Channel, Clock, DisplayName, MeetingId};

use super::SpeakerTagReader;
use super::align::{Timed, align};
use super::plan::{TagPlan, Track, plan};
use super::resolve::{Directory, resolve};
use super::seats::{Ballot, Seated, elect};
use crate::people::{AttendeeRepo, PersonRepo};
use crate::segments::{Page, Segment, SegmentReader, Speaker};

/// The meeting app's names as priors for naming a meeting's speakers.
#[async_trait]
pub trait TagPriors: Send + Sync {
    /// The plan for `meeting`, whose speakers are `speakers` now.
    async fn plan(&self, meeting: &MeetingId, speakers: &[Speaker]) -> Result<TagPlan, DbError>;
}

/// No speaker tags: every plan is empty (tests, and builds without them).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSpeakerTags;

#[async_trait]
impl TagPriors for NoSpeakerTags {
    async fn plan(&self, _meeting: &MeetingId, _speakers: &[Speaker]) -> Result<TagPlan, DbError> {
        Ok(TagPlan::default())
    }
}

/// The ports [`SpeakerTagPriors`] reads through.
#[derive(Clone)]
pub struct TagPriorPorts {
    /// The recorded spans.
    pub tags: Arc<dyn SpeakerTagReader>,
    /// The transcript.
    pub segments: Arc<dyn SegmentReader>,
    /// Who was invited, for resolving names.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// Everyone the user knows, for resolving names.
    pub people: Arc<dyn PersonRepo>,
    /// Stamps new speakers' IDs.
    pub clock: Arc<dyn Clock>,
}

/// [`TagPriors`] over the stored spans (see the module docs).
pub struct SpeakerTagPriors {
    ports: TagPriorPorts,
}

impl SpeakerTagPriors {
    /// Creates the priors over `ports`.
    pub fn new(ports: TagPriorPorts) -> Self {
        Self { ports }
    }
}

#[async_trait]
impl TagPriors for SpeakerTagPriors {
    async fn plan(&self, meeting: &MeetingId, speakers: &[Speaker]) -> Result<TagPlan, DbError> {
        let spans = self.ports.tags.spans(meeting).await?;
        if spans.is_empty() {
            return Ok(TagPlan::default());
        }
        let segments = system_track(self.ports.segments.list(meeting, Page::ALL).await?);
        let timed: Vec<Timed> = segments.iter().map(timed).collect();
        let wins = align(&spans, &timed);
        let directory = Directory {
            attendees: self.ports.attendees.as_ref(),
            people: self.ports.people.as_ref(),
        };
        let names: Vec<DisplayName> = wins
            .iter()
            .map(|win| win.name.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let people = resolve(&directory, meeting, &names).await?;
        let seated: Vec<Seated> = segments.iter().map(seated).collect();
        let ballot = Ballot {
            wins: &wins,
            segments: &seated,
            speakers,
            people: &people,
        };
        let election = elect(&ballot);
        let track = Track {
            meeting,
            speakers,
            segments: &seated,
        };
        let tag_plan = plan(&election, &track, self.ports.clock.as_ref());
        log::info!(
            target: "deino_core::speaker_tags",
            "meeting {meeting}: {} spans, {} of {} segments won, {} names ({} known), {} moved",
            spans.len(),
            wins.len(),
            seated.len(),
            names.len(),
            people.len(),
            tag_plan.seating.moved.len()
        );
        Ok(tag_plan)
    }
}

/// The segments heard on the system track, not deleted.
fn system_track(segments: Vec<Segment>) -> Vec<Segment> {
    segments
        .into_iter()
        .filter(|segment| segment.channel == Channel::System && !segment.is_deleted)
        .collect()
}

fn timed(segment: &Segment) -> Timed {
    Timed {
        id: segment.id,
        start: segment.start,
        end: segment.end,
    }
}

fn seated(segment: &Segment) -> Seated {
    Seated {
        id: segment.id,
        speaker: segment.speaker_id,
        length: segment.end.get().saturating_sub(segment.start.get()),
    }
}

#[cfg(test)]
#[path = "priors_tests.rs"]
mod tests;
