//! Scribe's words as utterances (pure): the transcript lines Deino stores.
//!
//! Scribe answers one list of words, spaces and events for the whole file.
//! A line ends where the voice changes, at a pause, or once it is long and
//! reaches the end of a sentence (or very long regardless), like the
//! local engine's utterances. Timings become milliseconds; events and
//! untimed words are left out.

use deino_core::final_transcription::{CloudSpan, HeardSpan};
use deino_core::segments::WordTiming;
use deino_domain::Millis;

use crate::wire::{ScribeWord, ScribeWordKind};

/// A silence this long starts a new line.
const PAUSE: Millis = Millis(1_000);
/// A line this long ends at the next sentence end.
const LONG: Millis = Millis(20_000);
/// A line this long ends at the next word.
const LONGEST: Millis = Millis(40_000);

/// The utterances in `words`, in time order. Voices are kept when
/// `diarized`, else every line has none.
pub fn utterances(words: &[ScribeWord], diarized: bool) -> Vec<CloudSpan> {
    let mut lines: Vec<CloudSpan> = Vec::new();
    for word in words
        .iter()
        .filter(|word| word.kind == ScribeWordKind::Word)
    {
        let Some(timing) = timing(word) else {
            continue;
        };
        let voice = word.speaker_id.clone().filter(|_| diarized);
        match lines.last_mut() {
            Some(line) if continues(line, &timing, voice.as_deref()) => push(line, timing),
            _ => lines.push(start(timing, voice)),
        }
    }
    lines
}

/// The word's text and times, if it is timed and not blank.
fn timing(word: &ScribeWord) -> Option<WordTiming> {
    let text = word.text.trim();
    let (start, end) = (millis(word.start?), millis(word.end?));
    (!text.is_empty()).then(|| WordTiming {
        text: text.to_owned(),
        start,
        end: end.max(start),
    })
}

/// Whether `word`, in `voice`, belongs to `line`.
fn continues(line: &CloudSpan, word: &WordTiming, voice: Option<&str>) -> bool {
    let span = &line.span;
    let length = span.end.0.saturating_sub(span.start.0);
    let sentence_ended = span.text.ends_with(['.', '?', '!']);
    line.voice.as_deref() == voice
        && word.start.0.saturating_sub(span.end.0) < PAUSE.0
        && !(length >= LONG.0 && sentence_ended)
        && length < LONGEST.0
}

fn start(word: WordTiming, voice: Option<String>) -> CloudSpan {
    let span = HeardSpan {
        start: word.start,
        end: word.end,
        text: word.text.clone(),
        words: vec![word],
    };
    CloudSpan { span, voice }
}

fn push(line: &mut CloudSpan, word: WordTiming) {
    let span = &mut line.span;
    span.end = span.end.max(word.end);
    span.text.push(' ');
    span.text.push_str(&word.text);
    span.words.push(word);
}

/// `seconds` as milliseconds, clamped to what [`Millis`] holds.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rounded and clamped to 0..=u32::MAX before the cast"
)]
fn millis(seconds: f64) -> Millis {
    let ms = (seconds * 1_000.0).round();
    if ms.is_nan() || ms <= 0.0 {
        Millis(0)
    } else {
        Millis(ms.min(f64::from(u32::MAX)) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, (start, end): (f64, f64), speaker: &str) -> ScribeWord {
        ScribeWord {
            text: text.into(),
            kind: ScribeWordKind::Word,
            start: Some(start),
            end: Some(end),
            speaker_id: Some(speaker.into()),
        }
    }

    fn space(at: f64) -> ScribeWord {
        ScribeWord {
            text: " ".into(),
            kind: ScribeWordKind::Spacing,
            start: Some(at),
            end: Some(at),
            speaker_id: None,
        }
    }

    fn texts(lines: &[CloudSpan]) -> Vec<(&str, Option<&str>)> {
        lines
            .iter()
            .map(|line| (line.span.text.as_str(), line.voice.as_deref()))
            .collect()
    }

    #[test]
    fn a_new_voice_or_a_pause_starts_a_new_line() {
        let words = [
            word("Hi", (0.0, 0.3), "speaker_0"),
            space(0.3),
            word("all.", (0.35, 0.6), "speaker_0"),
            word("Hey!", (0.7, 0.9), "speaker_1"),
            word("Later", (3.0, 3.4), "speaker_1"),
        ];

        let lines = utterances(&words, true);

        assert_eq!(
            texts(&lines),
            [
                ("Hi all.", Some("speaker_0")),
                ("Hey!", Some("speaker_1")),
                ("Later", Some("speaker_1"))
            ]
        );
        assert_eq!(
            (lines[0].span.start, lines[0].span.end),
            (Millis(0), Millis(600))
        );
        assert_eq!(lines[0].span.words.len(), 2);
    }

    #[test]
    fn undiarized_lines_have_no_voice_and_events_or_untimed_words_are_dropped() {
        let mut untimed = word("lost", (0.0, 0.0), "speaker_0");
        untimed.start = None;
        let event = ScribeWord {
            kind: ScribeWordKind::AudioEvent,
            ..word("(laughter)", (0.5, 0.9), "speaker_0")
        };
        let words = [word("Hello", (0.0, 0.4), "speaker_0"), untimed, event];

        assert_eq!(texts(&utterances(&words, false)), [("Hello", None)]);
    }

    #[test]
    fn a_long_line_ends_at_a_sentence_end() {
        let words = [
            word("One.", (0.0, 20.5), "speaker_0"),
            word("Two", (20.6, 21.0), "speaker_0"),
        ];

        assert_eq!(texts(&utterances(&words, true)).len(), 2);
        assert_eq!(millis(-1.0), Millis(0));
        assert_eq!(millis(1.2345), Millis(1_235));
    }
}
