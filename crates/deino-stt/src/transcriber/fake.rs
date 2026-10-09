//! An in-process transcriber for session tests (behind `test-support`).
//!
//! Decodes with the scripted engine's rules (every loud 400 ms block becomes
//! a word named after its block), and can lose its stream once it has heard
//! a given amount of audio, the way a crashed worker does.

use std::sync::{Arc, Mutex, PoisonError};

use super::{BatchTranscriber, StreamingTranscriber, TranscriptStream};
use crate::engine::scripted::{ScriptedDecoder, scripted_tokens};
use crate::error::SttError;
use crate::pcm::Millis;
use crate::protocol::{BatchTranscript, StreamDelta, StreamParams};
use crate::words::text_of;

/// What the fake has been asked to do, for assertions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FakeCalls {
    /// Streams opened.
    pub streams: usize,
    /// The length in samples of every batch run.
    pub batch_samples: Vec<usize>,
}

/// A scripted, in-process [`BatchTranscriber`] and [`StreamingTranscriber`].
#[derive(Debug, Default)]
pub struct FakeTranscriber {
    /// A doomed stream is lost by the feed that takes it past this much audio.
    lose_stream_after: Option<Millis>,
    /// Only the first `n` streams are lost, if set.
    lost_streams: Option<usize>,
    calls: Arc<Mutex<FakeCalls>>,
}

impl FakeTranscriber {
    /// A transcriber that never fails.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loses each of the first `streams` streams once it has heard more
    /// than `audio`.
    #[must_use]
    pub fn losing_streams_after(mut self, audio: Millis, streams: usize) -> Self {
        self.lose_stream_after = Some(audio);
        self.lost_streams = Some(streams);
        self
    }

    /// What was asked so far.
    pub fn calls(&self) -> FakeCalls {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl BatchTranscriber for FakeTranscriber {
    fn transcribe(&self, pcm: Vec<f32>) -> Result<BatchTranscript, SttError> {
        let mut calls = self.calls.lock().unwrap_or_else(PoisonError::into_inner);
        calls.batch_samples.push(pcm.len());
        let tokens = scripted_tokens(&pcm, 0);
        Ok(BatchTranscript {
            text: text_of(&tokens),
            tokens,
        })
    }
}

impl StreamingTranscriber for FakeTranscriber {
    fn open_stream(&self, _params: StreamParams) -> Result<Box<dyn TranscriptStream>, SttError> {
        let mut calls = self.calls.lock().unwrap_or_else(PoisonError::into_inner);
        calls.streams += 1;
        let doomed = self.lost_streams.is_some_and(|lost| calls.streams <= lost);
        Ok(Box::new(FakeStream {
            decoder: ScriptedDecoder::default(),
            heard: 0,
            lost: false,
            lose_after: self.lose_stream_after.filter(|_| doomed),
        }))
    }
}

struct FakeStream {
    decoder: ScriptedDecoder,
    /// Samples fed so far.
    heard: usize,
    lost: bool,
    lose_after: Option<Millis>,
}

impl TranscriptStream for FakeStream {
    fn feed(&mut self, pcm: Vec<f32>) -> Result<StreamDelta, SttError> {
        self.heard += pcm.len();
        self.lost |= self
            .lose_after
            .is_some_and(|audio| Millis::from_samples(self.heard) > audio);
        if self.lost {
            return Err(SttError::StreamLost);
        }
        Ok(self.decoder.feed(&pcm))
    }

    fn finalize(mut self: Box<Self>) -> Result<Option<StreamDelta>, SttError> {
        if self.lost {
            return Ok(None);
        }
        Ok(Some(self.decoder.finalize()))
    }
}
