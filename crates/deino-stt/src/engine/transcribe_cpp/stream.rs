//! A transcribe-cpp stream that reports only what changed: the tokens
//! committed since its last answer, and the tentative tail when it moved.

use transcribe_cpp::{Stream, StreamUpdate, Token};

use super::remote;
use crate::engine::EngineStream;
use crate::protocol::{RemoteError, RemoteErrorKind, StreamDelta, StreamProgress};
use crate::words::TimedToken;

/// A transcribe-cpp stream that reports token deltas.
pub struct CppStream<'a> {
    pub(super) stream: Stream<'a>,
    /// Committed tokens already sent to the app.
    pub(super) sent: usize,
}

impl EngineStream for CppStream<'_> {
    fn feed(&mut self, pcm: &[f32]) -> Result<StreamDelta, RemoteError> {
        let update = self
            .stream
            .feed(pcm)
            .map_err(|error| remote(RemoteErrorKind::Inference, &error))?;
        Ok(self.delta(update))
    }

    fn finalize(&mut self) -> Result<StreamDelta, RemoteError> {
        let update = self
            .stream
            .finalize()
            .map_err(|error| remote(RemoteErrorKind::Inference, &error))?;
        Ok(self.delta(update))
    }
}

impl CppStream<'_> {
    fn delta(&mut self, update: StreamUpdate) -> StreamDelta {
        let tokens = if update.committed_changed || update.is_final {
            self.new_committed_tokens(update.is_final)
        } else {
            Vec::new()
        };
        StreamDelta {
            progress: StreamProgress {
                received_ms: update.input_received_ms,
                committed_ms: update.audio_committed_ms,
                buffered_ms: update.buffered_ms,
            },
            tokens,
            tentative: update
                .tentative_changed
                .then(|| self.stream.text().tentative),
        }
    }

    /// The committed tokens not sent yet. The snapshot holds tentative tokens
    /// too; the committed text's length says how many of them are committed.
    fn new_committed_tokens(&mut self, is_final: bool) -> Vec<TimedToken> {
        let committed_len = self.stream.text().committed.len();
        let snapshot = self.stream.snapshot();
        let committed = if is_final {
            snapshot.tokens.len()
        } else {
            committed_token_count(&snapshot.tokens, committed_len)
        };
        let fresh = snapshot
            .tokens
            .get(self.sent..committed)
            .map(|tokens| tokens.iter().map(timed_token).collect())
            .unwrap_or_default();
        self.sent = self.sent.max(committed);
        fresh
    }
}

/// How many leading `tokens` the committed text covers. Committed text is the
/// tokens' concatenation without its leading space, hence the one-byte slack.
fn committed_token_count(tokens: &[Token], committed_len: usize) -> usize {
    let mut covered = 0_usize;
    tokens
        .iter()
        .take_while(|token| {
            covered += token.text.len();
            covered <= committed_len + 1
        })
        .count()
}

pub(super) fn timed_token(token: &Token) -> TimedToken {
    TimedToken {
        text: token.text.clone(),
        start_ms: token.t0_ms,
        end_ms: token.t1_ms,
        confidence: (!token.p.is_nan()).then_some(token.p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str) -> Token {
        Token {
            text: text.into(),
            ..Token::default()
        }
    }

    #[test]
    fn committed_tokens_are_counted_by_the_committed_text_length() {
        let tokens = [token(" Hello"), token(" there"), token(" frie")];

        assert_eq!(committed_token_count(&tokens, "Hello there".len()), 2);
        assert_eq!(committed_token_count(&tokens, 0), 0);
        assert_eq!(committed_token_count(&tokens, "Hello there frie".len()), 3);
    }

    #[test]
    fn a_nan_confidence_crosses_the_wire_as_none() {
        let mut nan = token(" a");
        nan.p = f32::NAN;

        assert_eq!(timed_token(&nan).confidence, None);
    }
}
