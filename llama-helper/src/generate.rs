//! The generation loop, independent of llama.cpp.
//!
//! [`drive`] pulls decoded pieces from a [`TokenSource`] (the model, or a
//! script in tests), runs them through a `StopScanner` and hands the text that
//! is safe to show to a [`TokenSink`]: a [`CollectSink`] for `generate`, or a
//! sink that writes `token` lines for `stream` and `grammar`. It stops at the
//! end-of-generation token, at `max_tokens` or at a stop token.

use anyhow::Result;

use crate::protocol::StopReason;
use crate::stop::{Scan, StopScanner};

/// One step of a model's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The next decoded piece; never a partial UTF-8 character, maybe empty.
    Piece(String),
    /// The model produced an end-of-generation token.
    EndOfGeneration,
}

/// Produces a model's output one token at a time.
pub trait TokenSource {
    /// Samples the next token and decodes it.
    fn next_step(&mut self) -> Result<Step>;
}

/// Receives the generated text as it becomes safe to show.
pub trait TokenSink {
    /// Takes the next non-empty piece of text.
    fn emit(&mut self, text: &str) -> Result<()>;
}

/// Collects the whole completion, for `generate`.
#[derive(Debug, Default)]
pub struct CollectSink {
    /// Everything emitted so far.
    pub text: String,
}

impl TokenSink for CollectSink {
    fn emit(&mut self, text: &str) -> Result<()> {
        self.text.push_str(text);
        Ok(())
    }
}

/// When to stop generating.
#[derive(Debug, Clone, Copy)]
pub struct Limits<'a> {
    /// Tokens to generate at most; zero or less generates nothing.
    pub max_tokens: i32,
    /// Texts that end generation and are removed from the output.
    pub stop_tokens: &'a [String],
}

/// How a generation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finish {
    /// Tokens generated, not counting the end-of-generation token.
    pub output_tokens: u32,
    /// Why generation ended.
    pub stop: StopReason,
}

/// Generates from `source` into `sink` until one of `limits` is reached.
pub fn drive(
    source: &mut impl TokenSource,
    limits: Limits<'_>,
    sink: &mut impl TokenSink,
) -> Result<Finish> {
    let mut scanner = StopScanner::new(limits.stop_tokens);
    let mut output_tokens: u32 = 0;
    let stop = loop {
        if i64::from(output_tokens) >= i64::from(limits.max_tokens) {
            break StopReason::MaxTokens;
        }
        let Step::Piece(piece) = source.next_step()? else {
            break StopReason::EndOfGeneration;
        };
        output_tokens = output_tokens.saturating_add(1);
        match scanner.push(&piece) {
            Scan::Continue(text) => emit_some(sink, &text)?,
            Scan::Stopped(text) => {
                emit_some(sink, &text)?;
                return Ok(Finish {
                    output_tokens,
                    stop: StopReason::StopToken,
                });
            }
        }
    };
    emit_some(sink, &scanner.finish())?;
    Ok(Finish {
        output_tokens,
        stop,
    })
}

/// Emits `text` unless it is empty.
fn emit_some(sink: &mut impl TokenSink, text: &str) -> Result<()> {
    if text.is_empty() {
        Ok(())
    } else {
        sink.emit(text)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    /// Plays back scripted pieces, then the end-of-generation token.
    struct Script(VecDeque<&'static str>);

    impl TokenSource for Script {
        fn next_step(&mut self) -> Result<Step> {
            Ok(self
                .0
                .pop_front()
                .map_or(Step::EndOfGeneration, |piece| Step::Piece(piece.to_owned())))
        }
    }

    /// Records every emitted piece separately.
    #[derive(Default)]
    struct Pieces(Vec<String>);

    impl TokenSink for Pieces {
        fn emit(&mut self, text: &str) -> Result<()> {
            self.0.push(text.to_owned());
            Ok(())
        }
    }

    fn run(pieces: &[&'static str], max_tokens: i32, stops: &[&str]) -> (Vec<String>, Finish) {
        let stop_tokens: Vec<String> = stops.iter().map(|&stop| stop.to_owned()).collect();
        let limits = Limits {
            max_tokens,
            stop_tokens: &stop_tokens,
        };
        let mut sink = Pieces::default();
        let finish = drive(
            &mut Script(pieces.iter().copied().collect()),
            limits,
            &mut sink,
        )
        .unwrap();
        (sink.0, finish)
    }

    #[test]
    fn ends_at_the_end_of_generation_token() {
        let (pieces, finish) = run(&["Good", "bye"], 10, &[]);
        assert_eq!(pieces, ["Good", "bye"]);
        assert_eq!(finish.output_tokens, 2);
        assert_eq!(finish.stop, StopReason::EndOfGeneration);
    }

    #[test]
    fn ends_at_max_tokens() {
        let (pieces, finish) = run(&["a", "b", "c"], 2, &[]);
        assert_eq!(pieces, ["a", "b"]);
        assert_eq!(finish.stop, StopReason::MaxTokens);
        let (pieces, _) = run(&["a"], 0, &[]);
        assert!(pieces.is_empty());
    }

    #[test]
    fn never_emits_any_part_of_a_split_stop_token() {
        let (pieces, finish) = run(
            &["Done", ".\n", "<end", "_of_turn", ">", "x"],
            10,
            &["<end_of_turn>"],
        );
        assert_eq!(pieces, ["Done", "."]);
        assert_eq!(finish.output_tokens, 5);
        assert_eq!(finish.stop, StopReason::StopToken);
    }

    #[test]
    fn releases_held_text_when_generation_ends_without_a_stop_token() {
        let (pieces, finish) = run(&["ok", " <end"], 2, &["<end_of_turn>"]);
        assert_eq!(pieces, ["ok", " <end"]);
        assert_eq!(finish.stop, StopReason::MaxTokens);
    }

    #[test]
    fn collect_sink_joins_the_text() {
        let mut sink = CollectSink::default();
        let limits = Limits {
            max_tokens: 10,
            stop_tokens: &["<|im_end|>".to_owned()],
        };
        let mut script = Script(["Sum", "mary ", "<|im_end|>"].into_iter().collect());
        drive(&mut script, limits, &mut sink).unwrap();
        assert_eq!(sink.text, "Summary");
    }
}
