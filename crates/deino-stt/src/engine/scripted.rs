//! A deterministic fake engine for tests (behind `test-support`).
//!
//! It cuts audio into [`WORD_MS`] blocks and turns every loud block into a
//! word named after the block (` w0`, ` w1`, ...), timed to the block, so a
//! test can predict exactly which words a lifecycle produces and when. A
//! stream commits a block once the audio after it has arrived, like a real
//! decoder with lookahead.
//!
//! A [`FaultPlan`] crashes or hangs the worker at a chosen stage, the way a
//! `GGML_ASSERT` abort or a wedged GPU driver would (Handy's
//! `HANDY_WORKER_FAULT`, ported).

mod fault;

use std::collections::HashMap;
use std::ffi::OsStr;

pub use fault::{FaultKind, FaultPlan, FaultStage};

use super::{Engine, EngineStream};
use crate::pcm::Millis;
use crate::protocol::{
    BatchParams, BatchTranscript, LoadRequest, LoadedInfo, RemoteError, RemoteErrorKind,
    StreamDelta, StreamParams, StreamProgress,
};
use crate::words::{TimedToken, text_of};

/// The length of one scripted word.
pub const WORD_MS: u64 = 400;

/// A block is a word when its peak amplitude reaches this.
const LOUD: f32 = 0.1;

/// The tokens the scripted engine hears in `pcm`, numbered from `first_block`.
pub fn scripted_tokens(pcm: &[f32], first_block: usize) -> Vec<TimedToken> {
    pcm.chunks(block_len())
        .enumerate()
        .filter(|(_, block)| block.iter().any(|sample| sample.abs() >= LOUD))
        .map(|(index, _)| word_token(first_block + index))
        .collect()
}

fn word_token(block: usize) -> TimedToken {
    let word_ms = i64::try_from(WORD_MS).unwrap_or(i64::MAX);
    let start = i64::try_from(block)
        .unwrap_or(i64::MAX)
        .saturating_mul(word_ms);
    TimedToken {
        text: format!(" w{block}"),
        start_ms: start,
        end_ms: start.saturating_add(word_ms),
        confidence: Some(0.9),
    }
}

/// The scripted engine.
#[derive(Debug, Default)]
pub struct ScriptedEngine {
    fault: Option<FaultPlan>,
    cpu_only: bool,
    loaded: bool,
    reached: HashMap<FaultStage, u32>,
}

impl ScriptedEngine {
    /// An engine that fails as `fault` says.
    pub fn new(fault: Option<FaultPlan>) -> Self {
        Self {
            fault,
            ..Self::default()
        }
    }

    /// Fires the planned fault if this is its stage and turn.
    fn reach(&mut self, stage: FaultStage) {
        let count = self.reached.entry(stage).or_default();
        *count += 1;
        let reached = *count;
        if let Some(plan) = &self.fault {
            plan.fire_if_due(stage, reached, self.cpu_only);
        }
    }
}

impl Engine for ScriptedEngine {
    type Stream<'a> = ScriptedStream<'a>;

    fn init(&mut self, cpu_only: bool) -> Result<(), RemoteError> {
        self.cpu_only = cpu_only;
        self.reach(FaultStage::Init);
        Ok(())
    }

    fn warm_gpu(&mut self) {}

    fn load(&mut self, request: &LoadRequest) -> Result<LoadedInfo, RemoteError> {
        self.loaded = false;
        self.reach(FaultStage::Load);
        if !request.path.exists() {
            return Err(RemoteError::new(
                RemoteErrorKind::Load,
                "model file not found",
            ));
        }
        self.loaded = true;
        Ok(LoadedInfo {
            arch: "scripted".into(),
            backend: if self.cpu_only { "CPU" } else { "GPU" }.into(),
            on_gpu: !self.cpu_only,
            supports_streaming: request.path.extension() != Some(OsStr::new("batch")),
            max_audio_ms: 0,
        })
    }

    fn run(&mut self, pcm: &[f32], _params: &BatchParams) -> Result<BatchTranscript, RemoteError> {
        self.require_loaded()?;
        self.reach(FaultStage::Run);
        let tokens = scripted_tokens(pcm, 0);
        Ok(BatchTranscript {
            text: text_of(&tokens),
            tokens,
        })
    }

    fn open_stream(&mut self, _params: &StreamParams) -> Result<ScriptedStream<'_>, RemoteError> {
        self.require_loaded()?;
        Ok(ScriptedStream {
            engine: self,
            decoder: ScriptedDecoder::default(),
        })
    }
}

impl ScriptedEngine {
    fn require_loaded(&self) -> Result<(), RemoteError> {
        if self.loaded {
            Ok(())
        } else {
            Err(RemoteError::new(
                RemoteErrorKind::NotLoaded,
                "load a model first",
            ))
        }
    }
}

/// A scripted stream inside the scripted engine.
pub struct ScriptedStream<'a> {
    engine: &'a mut ScriptedEngine,
    decoder: ScriptedDecoder,
}

impl EngineStream for ScriptedStream<'_> {
    fn feed(&mut self, pcm: &[f32]) -> Result<StreamDelta, RemoteError> {
        self.engine.reach(FaultStage::Feed);
        Ok(self.decoder.feed(pcm))
    }

    fn finalize(&mut self) -> Result<StreamDelta, RemoteError> {
        self.engine.reach(FaultStage::Finalize);
        Ok(self.decoder.finalize())
    }
}

/// The scripted streaming decoder: commits each block once the next block
/// has arrived (one block of lookahead), like a real decoder's right context.
#[derive(Debug, Default)]
pub struct ScriptedDecoder {
    audio: Vec<f32>,
    committed_blocks: usize,
}

impl ScriptedDecoder {
    /// Adds audio and commits every block whose successor is complete.
    pub fn feed(&mut self, pcm: &[f32]) -> StreamDelta {
        self.audio.extend_from_slice(pcm);
        let ready = (self.audio.len() / block_len()).saturating_sub(1);
        self.commit(ready)
    }

    /// Commits everything, a trailing partial block included.
    pub fn finalize(&mut self) -> StreamDelta {
        let all = self.audio.len().div_ceil(block_len());
        self.commit(all)
    }

    fn commit(&mut self, blocks: usize) -> StreamDelta {
        let start = self.committed_blocks.min(blocks);
        let end = (blocks * block_len()).min(self.audio.len());
        let span = self.audio.get(start * block_len()..end).unwrap_or_default();
        let tokens = scripted_tokens(span, start);
        self.committed_blocks = self.committed_blocks.max(blocks);
        let received = Millis::from_samples(self.audio.len());
        let committed = Millis::from_samples(self.committed_blocks * block_len()).min(received);
        let ms = |millis: Millis| i64::try_from(millis.0).unwrap_or(i64::MAX);
        StreamDelta {
            progress: StreamProgress {
                received_ms: ms(received),
                committed_ms: ms(committed),
                buffered_ms: ms(received.saturating_sub(committed)),
            },
            tokens,
            tentative: None,
        }
    }
}

fn block_len() -> usize {
    Millis(WORD_MS).to_samples()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loud(ms: u64) -> Vec<f32> {
        vec![0.5; Millis(ms).to_samples()]
    }

    fn silent(ms: u64) -> Vec<f32> {
        vec![0.0; Millis(ms).to_samples()]
    }

    #[test]
    fn loud_blocks_become_numbered_words_timed_to_their_block() {
        let pcm = [loud(400), silent(400), loud(400)].concat();

        let tokens = scripted_tokens(&pcm, 0);

        assert_eq!(text_of(&tokens), "w0 w2");
        assert_eq!((tokens[1].start_ms, tokens[1].end_ms), (800, 1200));
    }

    #[test]
    fn fault_specs_parse_with_an_optional_occurrence() {
        let plan = FaultPlan::parse("hang@feed#3").unwrap();

        assert_eq!(
            (plan.kind, plan.stage, plan.nth),
            (FaultKind::Hang, FaultStage::Feed, 3)
        );
        assert_eq!(FaultPlan::parse("crash@load").unwrap().nth, 1);
        assert!(FaultPlan::parse("melt@load").is_none());
        assert!(FaultPlan::parse("crash@nowhere").is_none());
    }

    #[test]
    fn a_stream_commits_a_block_once_the_next_has_arrived() {
        let mut decoder = ScriptedDecoder::default();

        let first = decoder.feed(&loud(400));
        let second = decoder.feed(&loud(400));
        let last = decoder.finalize();

        assert!(first.tokens.is_empty());
        assert_eq!(text_of(&second.tokens), "w0");
        assert_eq!(second.progress.committed_ms, 400);
        assert_eq!(text_of(&last.tokens), "w1");
    }
}
