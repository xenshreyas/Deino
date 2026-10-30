//! Word timings for lines the transcriber left untimed (pure rules, port).
//!
//! Splitting a line where the voice changes ([`super::attribute`]) needs a
//! time for every word. Some engines time only whole lines (Whisper through
//! transcribe-cpp), so before attributing, the stage asks a [`WordAligner`]
//! to time the planned track's untimed lines. Without one ([`NoWordAligner`])
//! or when it fails, the lines stay untimed and go whole to one voice, as
//! before. A line the user edited keeps no words: its text no longer
//! matches the audio.

use std::path::Path;

use async_trait::async_trait;
use deino_domain::{Channel, Millis, SegmentId};

use super::TrackError;
use crate::segments::{Segment, WordTiming};

/// One line to time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignLine {
    /// The segment it is.
    pub segment: SegmentId,
    /// Where it starts in the recording.
    pub start: Millis,
    /// Where it ends.
    pub end: Millis,
    /// What was said; the aligner times each whitespace-separated word.
    pub text: String,
}

/// Times the words of a track's lines.
#[async_trait]
pub trait WordAligner: Send + Sync {
    /// For each of `lines` of the track at `audio`, its words in order, or
    /// an empty list when the line could not be timed.
    async fn align(
        &self,
        audio: &Path,
        lines: &[AlignLine],
    ) -> Result<Vec<Vec<WordTiming>>, TrackError>;
}

/// No aligner: every line stays as the transcriber left it.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoWordAligner;

#[async_trait]
impl WordAligner for NoWordAligner {
    async fn align(
        &self,
        _audio: &Path,
        _lines: &[AlignLine],
    ) -> Result<Vec<Vec<WordTiming>>, TrackError> {
        Ok(Vec::new())
    }
}

/// The lines of `channel` an aligner should time: heard, untimed, unedited.
pub fn untimed(segments: &[Segment], channel: Channel) -> Vec<AlignLine> {
    segments
        .iter()
        .filter(|s| s.channel == channel && !s.is_deleted && s.edited_at.is_none())
        .filter(|s| s.words.is_empty() && !s.text.trim().is_empty())
        .map(|s| AlignLine {
            segment: s.id,
            start: s.start,
            end: s.end,
            text: s.text.clone(),
        })
        .collect()
}

/// `segments` with `channel`'s untimed lines timed by `aligner` where it
/// could; on failure, `segments` as they were.
pub async fn time_track(
    aligner: &dyn WordAligner,
    audio: &Path,
    segments: Vec<Segment>,
    channel: Channel,
) -> Vec<Segment> {
    let lines = untimed(&segments, channel);
    if lines.is_empty() {
        return segments;
    }
    match aligner.align(audio, &lines).await {
        Ok(timed) if timed.is_empty() => segments,
        Ok(timed) => {
            log::info!(target: "deino_core::diarization", "timed the words of {} of {} lines", timed.iter().filter(|w| !w.is_empty()).count(), lines.len());
            with_words(segments, &lines, timed)
        }
        Err(error) => {
            log::warn!(target: "deino_core::diarization", "the words could not be timed, so lines are not split: {error}");
            segments
        }
    }
}

/// `segments` with the timed words of `lines`, where the aligner timed
/// every word of a line.
pub fn with_words(
    mut segments: Vec<Segment>,
    lines: &[AlignLine],
    timed: Vec<Vec<WordTiming>>,
) -> Vec<Segment> {
    for (line, words) in lines.iter().zip(timed) {
        let complete = !words.is_empty() && words.len() == line.text.split_whitespace().count();
        if let Some(segment) = segments.iter_mut().find(|s| s.id == line.segment)
            && complete
        {
            segment.words = words;
        }
    }
    segments
}

#[cfg(test)]
#[path = "align_tests.rs"]
mod tests;
