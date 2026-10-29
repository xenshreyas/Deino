//! Word timings for a transcript the model timed by phrase only.
//!
//! Whisper (through transcribe-cpp) times segments of a few seconds, not
//! words, so a 30-second run came back as one untimed line: nothing after
//! it could split the line where the voice changes. Each segment's words
//! are spread over its span by their length here, as tokens that start a
//! word. A word's time is approximate inside its phrase; the phrase's
//! bounds, where speakers usually change, are the model's own.

use transcribe_cpp::{Segment, TimestampKind, Transcript};

use crate::words::TimedToken;

/// The transcript's own tokens, or words spread over its timed segments
/// when the model timed segments only.
pub(super) fn tokens_or_phrases(transcript: &Transcript) -> Option<Vec<TimedToken>> {
    let timed_phrases =
        transcript.tokens.is_empty() && transcript.timestamp_kind == TimestampKind::Segment;
    timed_phrases.then(|| transcript.segments.iter().flat_map(spread).collect())
}

/// `segment`'s words, each given a share of its span by its length.
fn spread(segment: &Segment) -> Vec<TimedToken> {
    let words: Vec<&str> = segment.text.split_whitespace().collect();
    let weight = |word: &&str| i64::try_from(word.chars().count()).unwrap_or(i64::MAX);
    let total = words.iter().map(weight).sum::<i64>().max(1);
    let span = segment.t1_ms.saturating_sub(segment.t0_ms).max(0);
    let at = |done: i64| segment.t0_ms + span.saturating_mul(done) / total;
    let mut done = 0;
    words
        .iter()
        .map(|word| {
            let start = at(done);
            done += weight(word);
            TimedToken {
                text: format!(" {word}"),
                start_ms: start,
                end_ms: at(done),
                confidence: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(t0_ms: i64, t1_ms: i64, text: &str) -> Segment {
        Segment {
            t0_ms,
            t1_ms,
            text: text.into(),
            ..Segment::default()
        }
    }

    #[test]
    fn words_share_their_phrase_by_length_and_phrases_keep_their_bounds() {
        let transcript = Transcript {
            timestamp_kind: TimestampKind::Segment,
            segments: vec![
                segment(0, 3_000, " It's my personal preference."),
                segment(3_000, 4_000, " No, actually"),
            ],
            ..Transcript::default()
        };

        let tokens = tokens_or_phrases(&transcript).unwrap();
        let spans: Vec<(&str, i64, i64)> = tokens
            .iter()
            .map(|t| (t.text.as_str(), t.start_ms, t.end_ms))
            .collect();

        assert_eq!(
            spans,
            [
                (" It's", 0, 480),
                (" my", 480, 720),
                (" personal", 720, 1_680),
                (" preference.", 1_680, 3_000),
                (" No,", 3_000, 3_272),
                (" actually", 3_272, 4_000),
            ]
        );
    }

    #[test]
    fn a_model_with_its_own_tokens_or_no_timings_is_left_alone() {
        let untimed = Transcript {
            timestamp_kind: TimestampKind::None,
            segments: vec![segment(0, 0, " hello")],
            ..Transcript::default()
        };

        assert_eq!(tokens_or_phrases(&untimed), None);
    }
}
