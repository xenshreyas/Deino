//! Engine tokens and the timed words built from them.
//!
//! Streaming snapshots carry token timestamps only (`Transcript::words` stays
//! empty), so words are grouped from tokens here. Rules measured in the STT
//! spike (`deino-spikes/stt-streaming/src/words.rs`):
//!
//! - a token whose text starts with a space starts a new word (`SentencePiece`);
//! - a punctuation-only token joins the previous word's text but never moves
//!   its end time: Parakeet often emits the sentence-final "." only when the
//!   next speech starts, possibly seconds later.

use serde::{Deserialize, Serialize};

use crate::pcm::Millis;

/// One engine token, with times relative to the start of its stream or run.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TimedToken {
    /// The token text, with a leading space when it starts a word.
    pub text: String,
    /// Start time in ms from the start of the stream or run.
    pub start_ms: i64,
    /// End time in ms from the start of the stream or run.
    pub end_ms: i64,
    /// The engine's confidence hint, when it gives one.
    pub confidence: Option<f32>,
}

impl TimedToken {
    /// Whether this token begins a new word.
    pub fn starts_word(&self) -> bool {
        self.text.starts_with(' ')
    }

    /// Whether this token is punctuation only, such as a late ".".
    pub fn is_punctuation(&self) -> bool {
        let text = self.text.trim();
        !text.is_empty() && text.chars().all(|c| c.is_ascii_punctuation())
    }
}

/// One word on a recording's timeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimedWord {
    /// The word, with any punctuation that follows it.
    pub text: String,
    /// When the word starts.
    pub start: Millis,
    /// When the word ends (its last spoken token, not trailing punctuation).
    pub end: Millis,
    /// The lowest token confidence in the word, when the engine gives one.
    pub confidence: Option<f32>,
}

/// Groups `tokens` into words, shifting their times by `offset` onto the
/// recording's timeline.
pub fn words_from_tokens(tokens: &[TimedToken], offset: Millis) -> Vec<TimedWord> {
    let mut words: Vec<TimedWord> = Vec::new();
    for token in tokens.iter().filter(|token| !token.text.is_empty()) {
        let begins = token.starts_word() || words.is_empty();
        match words.last_mut() {
            Some(word) if !begins || token.is_punctuation() => extend(word, token, offset),
            _ => words.push(start_word(token, offset)),
        }
    }
    words
}

/// The text of `tokens` as the engine spelled it, without outer whitespace.
pub fn text_of(tokens: &[TimedToken]) -> String {
    let text: String = tokens.iter().map(|token| token.text.as_str()).collect();
    text.trim().to_string()
}

fn start_word(token: &TimedToken, offset: Millis) -> TimedWord {
    TimedWord {
        text: token.text.trim_start().to_string(),
        start: offset + Millis::from_engine(token.start_ms),
        end: offset + Millis::from_engine(token.end_ms),
        confidence: token.confidence,
    }
}

fn extend(word: &mut TimedWord, token: &TimedToken, offset: Millis) {
    word.text.push_str(token.text.trim_start());
    if !token.is_punctuation() {
        word.end = offset + Millis::from_engine(token.end_ms);
    }
    word.confidence = match (word.confidence, token.confidence) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str, start_ms: i64, end_ms: i64) -> TimedToken {
        TimedToken {
            text: text.into(),
            start_ms,
            end_ms,
            confidence: Some(0.9),
        }
    }

    #[test]
    fn subword_tokens_join_and_late_punctuation_keeps_the_end_time() {
        let tokens = [
            token(" mach", 47_040, 47_120),
            token("in", 47_040, 47_120),
            token("es", 47_040, 47_200),
            token(".", 56_640, 56_720),
            token(" Af", 56_720, 56_800),
            token("ter", 56_800, 56_880),
        ];

        let words = words_from_tokens(&tokens, Millis(1000));

        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "machines.");
        assert_eq!(
            (words[0].start, words[0].end),
            (Millis(48_040), Millis(48_200))
        );
        assert_eq!(words[1].text, "After");
        assert_eq!(words[1].start, Millis(57_720));
    }

    #[test]
    fn a_first_token_without_a_space_still_starts_a_word() {
        let words = words_from_tokens(&[token("Hello", 0, 80)], Millis::ZERO);

        assert_eq!(words[0].text, "Hello");
    }

    #[test]
    fn the_lowest_token_confidence_wins_and_missing_ones_are_ignored() {
        let mut low = token("lo", 80, 160);
        low.confidence = Some(0.2);
        let mut unknown = token("w", 160, 240);
        unknown.confidence = None;

        let words = words_from_tokens(&[token(" be", 0, 80), low, unknown], Millis::ZERO);

        assert_eq!(words[0].confidence, Some(0.2));
    }

    #[test]
    fn text_keeps_the_engine_spacing() {
        let tokens = [
            token(" Hi", 0, 80),
            token(",", 80, 160),
            token(" you", 160, 240),
        ];

        assert_eq!(text_of(&tokens), "Hi, you");
    }
}
