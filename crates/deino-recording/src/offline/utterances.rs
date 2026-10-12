//! Cutting a finished track into utterances for the batch engine (pure).
//!
//! The track's voice spans (the lanes' detector: earshot plus a hangover)
//! become utterances, each widened by [`PAD`] so no word is clipped, and cut
//! into pieces of at most [`MAX_UTTERANCE`]: the STT spike measured that a
//! batch run over a long clip can stop emitting text partway through, and
//! the supervisor's deadline grows with the audio it is given.

use std::ops::Range;

use deino_stt::Millis;

use crate::vad::{LaneVad, VoiceChange, VoiceSpan};

/// The longest stretch of audio one batch run gets.
pub const MAX_UTTERANCE: Millis = Millis(30_000);

/// Audio kept on each side of a voice span.
pub const PAD: Millis = Millis(200);

/// The voice spans of 16 kHz `pcm`, in order.
pub fn voice_spans(pcm: &[f32]) -> Vec<VoiceSpan> {
    let mut vad = LaneVad::default();
    let mut spans = Vec::new();
    let mut ended = |change| {
        if let VoiceChange::Ended(span) = change {
            spans.push(span);
        }
    };
    vad.push(pcm, &mut ended);
    if let Some(change) = vad.finish() {
        ended(change);
    }
    spans
}

/// The utterances of a track `length` long with `spans` of voice: each span
/// padded, clamped to the track, overlaps merged, then cut to at most
/// [`MAX_UTTERANCE`].
pub fn utterances(spans: &[VoiceSpan], length: Millis) -> Vec<Range<Millis>> {
    let mut merged: Vec<Range<Millis>> = Vec::new();
    for span in spans {
        let start = span.start.saturating_sub(PAD);
        let end = Millis((span.end.0 + PAD.0).min(length.0));
        match merged.last_mut() {
            Some(last) if start <= last.end => last.end = last.end.max(end),
            _ if end > start => merged.push(start..end),
            _ => {}
        }
    }
    merged.into_iter().flat_map(cut).collect()
}

/// `range` in pieces of at most [`MAX_UTTERANCE`].
fn cut(range: Range<Millis>) -> Vec<Range<Millis>> {
    let mut pieces = Vec::new();
    let mut start = range.start;
    while start < range.end {
        let end = Millis((start.0 + MAX_UTTERANCE.0).min(range.end.0));
        pieces.push(start..end);
        start = end;
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: u64, end: u64) -> VoiceSpan {
        VoiceSpan {
            start: Millis(start),
            end: Millis(end),
        }
    }

    #[test]
    fn spans_are_padded_merged_and_clamped_to_the_track() {
        let spans = [span(100, 1_000), span(1_300, 2_000), span(5_000, 5_950)];

        let cut = utterances(&spans, Millis(6_000));

        assert_eq!(
            cut,
            [Millis(0)..Millis(2_200), Millis(4_800)..Millis(6_000)]
        );
    }

    #[test]
    fn a_long_monologue_is_cut_into_runs_the_engine_can_finish() {
        let cut = utterances(&[span(0, 65_000)], Millis(70_000));

        let lengths: Vec<u64> = cut.iter().map(|r| r.end.0 - r.start.0).collect();
        assert_eq!(lengths, [30_000, 30_000, 5_200]);
    }

    #[test]
    fn silence_has_no_utterances() {
        assert!(voice_spans(&vec![0.0; 32_000]).is_empty());
        assert!(utterances(&[], Millis(10_000)).is_empty());
    }
}
