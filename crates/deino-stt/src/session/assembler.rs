//! Turns a stream's committed tokens into transcript segments.
//!
//! Committed tokens never change, so segments are built incrementally: the
//! open segment grows token by token and closes at a pause, after a long
//! stretch of silence the decoder has already passed, or when it gets long.
//! Tokens here carry absolute times (ms on the recording's timeline).

use crate::pcm::Millis;
use crate::words::TimedToken;

/// When an open segment closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentPolicy {
    /// A gap this long between two words starts a new segment.
    pub pause: Millis,
    /// The decoder passing this much silence after the last word closes the segment.
    pub silence_close: Millis,
    /// Past this length, a segment closes after the next sentence end.
    pub soft_max: Millis,
    /// Past this length, a segment closes at the next word.
    pub hard_max: Millis,
}

impl Default for SegmentPolicy {
    fn default() -> Self {
        Self {
            pause: Millis(700),
            silence_close: Millis(1200),
            soft_max: Millis(15_000),
            hard_max: Millis(30_000),
        }
    }
}

/// A closed segment's tokens, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Closed(pub Vec<TimedToken>);

/// What one push changed.
#[derive(Debug, Default, PartialEq)]
pub struct Pushed {
    /// Segments closed by the push.
    pub closed: Vec<Closed>,
    /// Start times of segments the push opened.
    pub opened: Vec<i64>,
}

/// The open segment and the rules that close it.
#[derive(Debug)]
pub struct Assembler {
    policy: SegmentPolicy,
    open: Vec<TimedToken>,
    /// End of the open segment's last spoken (non-punctuation) token.
    spoken_end: i64,
}

impl Assembler {
    /// An empty assembler.
    pub fn new(policy: SegmentPolicy) -> Self {
        Self {
            policy,
            open: Vec::new(),
            spoken_end: 0,
        }
    }

    /// Adds newly committed tokens.
    pub fn push(&mut self, tokens: Vec<TimedToken>) -> Pushed {
        let mut pushed = Pushed::default();
        for token in tokens {
            self.push_one(token, &mut pushed);
        }
        pushed
    }

    fn push_one(&mut self, token: TimedToken, pushed: &mut Pushed) {
        if token.is_punctuation() || (!token.starts_word() && !self.open.is_empty()) {
            // Joins the open word. Punctuation arriving after its segment
            // closed is dropped rather than starting a segment of its own.
            if !self.open.is_empty() {
                self.attach(token);
            }
            return;
        }
        if self.closes_before(&token) {
            pushed.closed.extend(self.take());
        }
        if self.open.is_empty() {
            pushed.opened.push(token.start_ms);
        }
        self.attach(token);
    }

    fn attach(&mut self, token: TimedToken) {
        if !token.is_punctuation() {
            self.spoken_end = token.end_ms;
        }
        self.open.push(token);
    }

    /// Whether `token`, which starts a word, belongs in a new segment.
    fn closes_before(&self, token: &TimedToken) -> bool {
        let Some(first) = self.open.first() else {
            return false;
        };
        let gap = Millis::from_engine(token.start_ms - self.spoken_end);
        let length = Millis::from_engine(self.spoken_end - first.start_ms);
        let sentence_ended = self
            .open
            .last()
            .is_some_and(|last| last.text.trim_end().ends_with(['.', '?', '!']));
        gap >= self.policy.pause
            || length >= self.policy.hard_max
            || (length >= self.policy.soft_max && sentence_ended)
    }

    /// The decoder has finished with audio up to `decoded`: closes the open
    /// segment when that much silence followed its last word.
    pub fn advance(&mut self, decoded: Millis) -> Option<Closed> {
        let silence = decoded.saturating_sub(Millis::from_engine(self.spoken_end));
        if self.open.is_empty() || silence < self.policy.silence_close {
            return None;
        }
        self.take()
    }

    /// Closes the open segment, if any.
    pub fn flush(&mut self) -> Option<Closed> {
        self.take()
    }

    /// Drops the open segment (its audio is being transcribed again).
    pub fn discard_open(&mut self) {
        self.open.clear();
    }

    /// The open segment's tokens.
    pub fn open(&self) -> &[TimedToken] {
        &self.open
    }

    fn take(&mut self) -> Option<Closed> {
        (!self.open.is_empty()).then(|| Closed(std::mem::take(&mut self.open)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::text_of;

    fn word(text: &str, start_ms: i64) -> TimedToken {
        TimedToken {
            text: format!(" {text}"),
            start_ms,
            end_ms: start_ms + 200,
            confidence: None,
        }
    }

    fn texts(closed: &[Closed]) -> Vec<String> {
        closed.iter().map(|segment| text_of(&segment.0)).collect()
    }

    #[test]
    fn a_pause_between_words_starts_a_new_segment() {
        let mut assembler = Assembler::new(SegmentPolicy::default());

        let pushed = assembler.push(vec![word("one", 0), word("two", 300), word("three", 1300)]);

        assert_eq!(texts(&pushed.closed), ["one two"]);
        assert_eq!(pushed.opened, [0, 1300]);
        assert_eq!(text_of(assembler.open()), "three");
    }

    #[test]
    fn late_punctuation_joins_the_open_segment_and_never_starts_one() {
        let mut assembler = Assembler::new(SegmentPolicy::default());
        let period = TimedToken {
            text: ".".into(),
            start_ms: 5000,
            end_ms: 5080,
            confidence: None,
        };

        let pushed = assembler.push(vec![word("done", 0), period.clone(), word("next", 5100)]);

        assert_eq!(texts(&pushed.closed), ["done."]);
        assert!(assembler.flush().is_some());
        assert!(assembler.push(vec![period]).opened.is_empty());
        assert!(assembler.open().is_empty());
    }

    #[test]
    fn decoded_silence_closes_the_open_segment() {
        let mut assembler = Assembler::new(SegmentPolicy::default());
        assembler.push(vec![word("hello", 0)]);

        assert_eq!(assembler.advance(Millis(1000)), None);
        let closed = assembler.advance(Millis(1400)).unwrap();

        assert_eq!(text_of(&closed.0), "hello");
        assert_eq!(assembler.advance(Millis(9000)), None);
    }

    #[test]
    fn a_long_segment_closes_at_a_sentence_end_then_at_any_word() {
        let policy = SegmentPolicy {
            soft_max: Millis(1000),
            hard_max: Millis(2000),
            ..SegmentPolicy::default()
        };
        let mut assembler = Assembler::new(policy);
        let words: Vec<_> = (0..12)
            .map(|i| word(if i == 6 { "end." } else { "w" }, i * 250))
            .collect();

        let pushed = assembler.push(words);

        // Closes after "end." (past the soft limit), then at the hard limit.
        assert_eq!(texts(&pushed.closed), ["w w w w w w end."]);
        assert_eq!(pushed.opened, [0, 1750]);
    }
}
