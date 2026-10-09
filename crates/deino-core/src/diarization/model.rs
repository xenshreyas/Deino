//! What the diarize stage asks of a track and gets back.

use std::path::PathBuf;

use deino_domain::{Channel, Millis};

/// How many speakers the calendar expects on the diarized track.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeakerHint {
    /// No calendar information: the engine's threshold decides.
    #[default]
    Unknown,
    /// No more than this many (some invitees may not have joined).
    AtMost(u16),
    /// This many: every invitee counted accepted the invitation.
    Exactly(u16),
}

/// One track to diarize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackRequest {
    /// The finalized audio file (`system.m4a`, `mic.m4a` or `audio.mp4`).
    pub audio: PathBuf,
    /// The expected speaker count.
    pub speakers: SpeakerHint,
}

/// A stretch of one voice's speech on the track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceTurn {
    /// Start, from the start of the recording.
    pub start: Millis,
    /// End; after `start`.
    pub end: Millis,
    /// Index into [`TrackDiarization::voices`].
    pub voice: usize,
}

/// One voice the track holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Voice {
    /// The voice's mean speaker embedding, L2-normalised.
    pub centroid: Vec<f32>,
    /// Total length of its turns.
    pub speech: Millis,
}

/// Who spoke when on one track.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackDiarization {
    /// Turns in time order.
    pub turns: Vec<VoiceTurn>,
    /// Voices in order of first appearance ("Speaker 1" first).
    pub voices: Vec<Voice>,
}

/// What one run of the stage did, for the pipeline's log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiarizationOutcome {
    /// The channel that was diarized, if any.
    pub channel: Option<Channel>,
    /// Speakers found on it.
    pub speakers: usize,
    /// Segments re-attributed (the user's included).
    pub segments: usize,
    /// Segments split where the speaker changed.
    pub splits: usize,
}
