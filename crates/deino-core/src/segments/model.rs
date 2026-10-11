//! Transcript segments and speakers as the transcript sees them.

use chrono::{DateTime, Utc};
use deino_domain::{Channel, MeetingId, Millis, PaletteColor, PersonId, SegmentId, SpeakerId};
use serde::{Deserialize, Serialize};

/// One utterance of a meeting's transcript.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    /// The segment's ID.
    pub id: SegmentId,
    /// The meeting it belongs to.
    pub meeting_id: MeetingId,
    /// Which audio it was heard on.
    pub channel: Channel,
    /// Who said it, once diarization or the user knows.
    pub speaker_id: Option<SpeakerId>,
    /// Offset of its first word from the start of the recording.
    pub start: Millis,
    /// Offset of its end from the start of the recording; never before `start`.
    pub end: Millis,
    /// What was said.
    pub text: String,
    /// Per-word timing, when the engine provides it.
    pub words: Vec<WordTiming>,
    /// Hidden from the transcript by the user (TRN-02); kept for undo.
    pub is_deleted: bool,
    /// When the user last edited `text`.
    pub edited_at: Option<DateTime<Utc>>,
    /// What the engine heard, kept by the first edit (TRN-02); `None` while
    /// the segment was never edited.
    pub original_text: Option<String>,
}

/// A segment to store, as the transcription worker produces it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSegment {
    /// The new segment's ID.
    pub id: SegmentId,
    /// The meeting it belongs to.
    pub meeting_id: MeetingId,
    /// Which audio it was heard on.
    pub channel: Channel,
    /// Who said it, if already known.
    pub speaker_id: Option<SpeakerId>,
    /// Offset of its first word from the start of the recording.
    pub start: Millis,
    /// Offset of its end; never before `start`.
    pub end: Millis,
    /// What was said.
    pub text: String,
    /// Per-word timing; empty when the engine has none.
    pub words: Vec<WordTiming>,
}

/// When one word was said. Stored as JSON in `transcript_segments.words_json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WordTiming {
    /// The word as transcribed.
    pub text: String,
    /// Offset of its start from the start of the recording.
    pub start: Millis,
    /// Offset of its end from the start of the recording.
    pub end: Millis,
}

/// A window of a long list: `limit` rows after skipping `offset`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page {
    /// The most rows to return.
    pub limit: u32,
    /// How many rows to skip.
    pub offset: u32,
}

impl Page {
    /// Every row: no limit, no offset.
    pub const ALL: Self = Self {
        limit: u32::MAX,
        offset: 0,
    };
}

/// A voice in one meeting's transcript.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Speaker {
    /// The speaker's ID.
    pub id: SpeakerId,
    /// The meeting the voice was heard in.
    pub meeting_id: MeetingId,
    /// What the transcript shows until a person is assigned ("Speaker 2");
    /// unique within the meeting.
    pub label: String,
    /// The person this voice belongs to, once identified.
    pub person_id: Option<PersonId>,
    /// True for the user's own voice.
    pub is_me: bool,
    /// The speaker's colour in the transcript.
    pub color: PaletteColor,
}

/// The voice a diarized speaker was clustered from (CAP-03), for voiceprint
/// matching (CAP-04). Never sent to the UI.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeakerVoice {
    /// The mean speaker embedding, L2-normalised.
    pub centroid: Vec<f32>,
    /// The model that produced it; vectors of different models never meet.
    pub embedding_model: String,
    /// Total length of the speaker's turns.
    pub speech: Millis,
}

/// A speaker to store, with the voice diarization found for it.
#[derive(Clone, Debug, PartialEq)]
pub struct NewSpeaker {
    /// The speaker row.
    pub speaker: Speaker,
    /// Its voice, when diarization produced it.
    pub voice: Option<SpeakerVoice>,
}

/// One stretch of a segment after attribution: who said it, when, what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentPiece {
    /// Who said it.
    pub speaker_id: Option<SpeakerId>,
    /// Start, from the start of the recording.
    pub start: Millis,
    /// End; never before `start`.
    pub end: Millis,
    /// What was said in this stretch.
    pub text: String,
    /// The stretch's word timings.
    pub words: Vec<WordTiming>,
}

/// A segment re-attributed to speakers. The first piece keeps the segment's
/// ID (and anything that points at it); each further piece becomes a new
/// segment on the same meeting and channel, with the ID beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reattribution {
    /// The segment.
    pub id: SegmentId,
    /// Its first (often only) piece.
    pub head: SegmentPiece,
    /// The pieces split off after a speaker change, in time order.
    pub tail: Vec<(SegmentId, SegmentPiece)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_timings_store_as_compact_camel_case_json() {
        let word = WordTiming {
            text: "hello".into(),
            start: Millis(1_250),
            end: Millis(1_600),
        };

        let json = serde_json::to_string(&word).unwrap();

        assert_eq!(json, r#"{"text":"hello","start":1250,"end":1600}"#);
        assert_eq!(serde_json::from_str::<WordTiming>(&json).unwrap(), word);
    }
}
