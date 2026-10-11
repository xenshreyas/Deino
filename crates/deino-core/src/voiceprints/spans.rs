//! Clean single-speaker spans to enrol voiceprints from.
//!
//! Adapted from anarlog `crates/voiceprint/src/lib.rs` (MIT). Words of one
//! speaker merge into runs across short gaps; any stretch where another
//! speaker on the same channel talks is cut out (overlapped speech
//! contaminates embeddings); long runs keep their middle (edges carry
//! hesitation); each speaker keeps its longest spans. The spike enrolled each
//! person from their 3 longest overlap-free spans of at least 1.5 s.

use deino_domain::Millis;

/// One timed word of one channel's transcript, with its speaker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanWord<S> {
    /// Start of the word.
    pub start: Millis,
    /// End of the word.
    pub end: Millis,
    /// Who said it.
    pub speaker: S,
}

/// A span chosen for enrolment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectedSpan<S> {
    /// Whose voice it is.
    pub speaker: S,
    /// Start of the span.
    pub start: Millis,
    /// End of the span.
    pub end: Millis,
    /// Length relative to the longest allowed span, in `[0, 1]`.
    pub quality: f32,
}

/// How spans are chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanConfig {
    /// Words of one speaker closer than this join one run.
    pub merge_gap: Millis,
    /// Shorter spans are dropped.
    pub min_span: Millis,
    /// Longer spans keep their middle this long.
    pub max_span: Millis,
    /// Spans kept per speaker, longest first.
    pub max_spans_per_speaker: usize,
}

impl Default for SpanConfig {
    fn default() -> Self {
        Self {
            merge_gap: Millis(400),
            min_span: Millis(1_500),
            max_span: Millis(10_000),
            max_spans_per_speaker: 3,
        }
    }
}

/// A run of one speaker's speech, `[start, end)` in milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run<S> {
    speaker: S,
    start: u32,
    end: u32,
}

/// The longest clean spans of each speaker in `words` (one channel), grouped
/// by speaker in key order, longest first within a speaker.
pub fn select_speaker_spans<S: Copy + Ord>(
    words: &[SpanWord<S>],
    config: &SpanConfig,
) -> Vec<SelectedSpan<S>> {
    let runs = merge_runs(words, config.merge_gap.get());
    let mut speakers: Vec<S> = runs.iter().map(|run| run.speaker).collect();
    speakers.dedup();
    speakers
        .into_iter()
        .flat_map(|speaker| spans_of(speaker, &runs, config))
        .collect()
}

/// One speaker's spans: their runs minus everyone else's, clipped, filtered,
/// longest first.
fn spans_of<S: Copy + Ord>(
    speaker: S,
    runs: &[Run<S>],
    config: &SpanConfig,
) -> Vec<SelectedSpan<S>> {
    let others: Vec<Run<S>> = runs
        .iter()
        .filter(|run| run.speaker != speaker)
        .copied()
        .collect();
    let mut spans: Vec<SelectedSpan<S>> = runs
        .iter()
        .filter(|run| run.speaker == speaker)
        .flat_map(|run| subtract(*run, &others))
        .map(|run| clip(run, config.max_span.get()))
        .filter(|run| run.end - run.start >= config.min_span.get())
        .map(|run| SelectedSpan {
            speaker,
            start: Millis(run.start),
            end: Millis(run.end),
            quality: quality(&run, config.max_span.get()),
        })
        .collect();
    spans.sort_by_key(|span| std::cmp::Reverse(span.end.get() - span.start.get()));
    spans.truncate(config.max_spans_per_speaker);
    spans
}

/// Each speaker's words merged across gaps of at most `merge_gap` ms,
/// sorted by speaker then start.
fn merge_runs<S: Copy + Ord>(words: &[SpanWord<S>], merge_gap: u32) -> Vec<Run<S>> {
    let mut sorted: Vec<&SpanWord<S>> = words.iter().filter(|word| word.end > word.start).collect();
    sorted.sort_by_key(|word| (word.speaker, word.start));
    let mut runs: Vec<Run<S>> = Vec::new();
    for word in sorted {
        let (start, end) = (word.start.get(), word.end.get());
        match runs.last_mut() {
            Some(run)
                if run.speaker == word.speaker && start.saturating_sub(run.end) <= merge_gap =>
            {
                run.end = run.end.max(end);
            }
            _ => runs.push(Run {
                speaker: word.speaker,
                start,
                end,
            }),
        }
    }
    runs
}

/// `run` with every overlap with `others` removed.
fn subtract<S: Copy>(run: Run<S>, others: &[Run<S>]) -> Vec<Run<S>> {
    let mut pieces = vec![run];
    for other in others {
        pieces = pieces
            .into_iter()
            .flat_map(|piece| {
                if other.end <= piece.start || other.start >= piece.end {
                    return vec![piece];
                }
                let before = (other.start > piece.start).then_some(Run {
                    end: other.start,
                    ..piece
                });
                let after = (other.end < piece.end).then_some(Run {
                    start: other.end,
                    ..piece
                });
                before.into_iter().chain(after).collect()
            })
            .collect();
    }
    pieces
}

/// The middle `max_span` ms of a longer run.
fn clip<S>(run: Run<S>, max_span: u32) -> Run<S> {
    let length = run.end - run.start;
    if length <= max_span {
        return run;
    }
    let start = run.start + (length - max_span) / 2;
    Run {
        start,
        end: start + max_span,
        ..run
    }
}

fn quality<S>(run: &Run<S>, max_span: u32) -> f32 {
    #[allow(clippy::cast_precision_loss, reason = "spans are at most hours in ms")]
    let ratio = (run.end - run.start) as f32 / max_span.max(1) as f32;
    ratio.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(start: u32, end: u32, speaker: u8) -> SpanWord<u8> {
        SpanWord {
            start: Millis(start),
            end: Millis(end),
            speaker,
        }
    }

    fn bounds(spans: &[SelectedSpan<u8>]) -> Vec<(u32, u32)> {
        spans
            .iter()
            .map(|span| (span.start.get(), span.end.get()))
            .collect()
    }

    #[test]
    fn words_across_short_gaps_merge_into_one_span() {
        let words = [
            word(0, 900, 0),
            word(1_100, 2_000, 0),
            word(2_200, 3_100, 0),
        ];
        assert_eq!(
            bounds(&select_speaker_spans(&words, &SpanConfig::default())),
            [(0, 3_100)]
        );
    }

    #[test]
    fn long_silences_split_spans_and_short_pieces_are_dropped() {
        let words = [word(0, 2_000, 0), word(8_000, 8_600, 0)];
        assert_eq!(
            bounds(&select_speaker_spans(&words, &SpanConfig::default())),
            [(0, 2_000)]
        );
    }

    #[test]
    fn overlapped_speech_is_cut_out() {
        let words = [word(0, 6_000, 0), word(3_500, 4_000, 1)];
        let spans = select_speaker_spans(&words, &SpanConfig::default());
        assert_eq!(bounds(&spans), [(0, 3_500), (4_000, 6_000)]);
        assert!(spans.iter().all(|span| span.speaker == 0));
    }

    #[test]
    fn long_spans_keep_their_middle_and_speakers_keep_their_longest() {
        let config = SpanConfig {
            max_spans_per_speaker: 1,
            ..SpanConfig::default()
        };
        let spans = select_speaker_spans(&[word(0, 30_000, 3), word(40_000, 42_000, 3)], &config);
        assert_eq!(bounds(&spans), [(10_000, 20_000)]);
        assert!((spans[0].quality - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn zero_length_words_are_ignored() {
        assert!(select_speaker_spans(&[word(100, 100, 0)], &SpanConfig::default()).is_empty());
    }
}
