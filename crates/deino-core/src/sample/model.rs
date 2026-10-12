//! The bundled sample meeting as typed values.

use deino_domain::{Channel, Millis, PaletteColor};
use serde::Deserialize;

use super::SampleError;

/// The resource, compiled into the binary so release builds always have it.
const BUNDLED: &str = include_str!("sample-meeting.json");

/// The sample meeting as bundled: what the importer writes.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SampleMeeting {
    /// The meeting's title; starts with "Sample" so the list shows what it is.
    pub title: String,
    /// How long the meeting ran.
    pub duration_ms: Millis,
    /// Who spoke; labels are unique.
    pub speakers: Vec<SampleSpeaker>,
    /// What was said, in order.
    pub segments: Vec<SampleSegment>,
    /// The enhanced notes, as Markdown.
    pub notes: String,
    /// The meeting's action items.
    pub action_items: Vec<SampleActionItem>,
}

/// One voice in the sample's transcript.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SampleSpeaker {
    /// The name the transcript shows. No person is created for it.
    pub label: String,
    /// True for the user's own voice.
    pub is_me: bool,
    /// The speaker's palette slot.
    pub color: PaletteColor,
}

/// One transcript segment of the sample.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SampleSegment {
    /// The [`SampleSpeaker::label`] of who said it.
    pub speaker: String,
    /// Which audio it was heard on.
    pub channel: Channel,
    /// Offset of its start.
    pub start_ms: Millis,
    /// Offset of its end; never before the start.
    pub end_ms: Millis,
    /// What was said.
    pub text: String,
}

/// One action item of the sample.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SampleActionItem {
    /// What to do.
    pub title: String,
    /// A sentence of context.
    pub description: String,
    /// The index in [`SampleMeeting::segments`] it was said in.
    pub segment: usize,
}

impl SampleMeeting {
    /// The bundled sample, parsed and checked.
    pub fn bundled() -> Result<Self, SampleError> {
        Self::parse(BUNDLED)
    }

    /// Parses `json` and checks it: every segment's speaker exists and its
    /// end is not before its start, and every action item points at a segment.
    pub fn parse(json: &str) -> Result<Self, SampleError> {
        let sample: Self = serde_json::from_str(json)?;
        sample.check()?;
        Ok(sample)
    }

    fn check(&self) -> Result<(), SampleError> {
        let known = |label: &str| self.speakers.iter().any(|speaker| speaker.label == label);
        if !self.segments.iter().all(|segment| known(&segment.speaker)) {
            return Err(SampleError::Invalid("a segment names an unknown speaker"));
        }
        if self
            .segments
            .iter()
            .any(|segment| segment.end_ms < segment.start_ms)
        {
            return Err(SampleError::Invalid("a segment ends before it starts"));
        }
        if self
            .action_items
            .iter()
            .any(|item| item.segment >= self.segments.len())
        {
            return Err(SampleError::Invalid(
                "an action item points past the transcript",
            ));
        }
        Ok(())
    }
}
