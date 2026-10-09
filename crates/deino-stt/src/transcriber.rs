//! The parent-side engine contracts a session depends on.
//!
//! Batch and streaming are separate traits (interface segregation); a live
//! session needs both, because it re-transcribes retained audio in one run
//! when a stream is lost. [`EngineSupervisor`] implements them over a worker
//! process; `FakeTranscriber` implements them in-process for tests.

#[cfg(any(test, feature = "test-support"))]
mod fake;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeTranscriber;

use crate::error::SttError;
use crate::protocol::{BatchTranscript, StreamDelta, StreamParams};
use crate::supervisor::{EngineSupervisor, StreamHandle};

/// Whole-buffer transcription (import, retranscribe, stream recovery).
pub trait BatchTranscriber: Send + Sync {
    /// Transcribes 16 kHz mono PCM; token times are relative to its start.
    fn transcribe(&self, pcm: Vec<f32>) -> Result<BatchTranscript, SttError>;
}

/// Live, incremental transcription.
pub trait StreamingTranscriber: Send + Sync {
    /// Opens a stream; the model holds one stream at a time.
    fn open_stream(&self, params: StreamParams) -> Result<Box<dyn TranscriptStream>, SttError>;
}

/// An open stream: feed audio in order, then finalize once.
pub trait TranscriptStream: Send {
    /// Feeds 16 kHz mono audio. [`SttError::StreamLost`] means the stream is
    /// gone and its uncommitted audio must be transcribed again.
    fn feed(&mut self, pcm: Vec<f32>) -> Result<StreamDelta, SttError>;

    /// Flushes the stream; `Ok(None)` when it was lost on the way.
    fn finalize(self: Box<Self>) -> Result<Option<StreamDelta>, SttError>;
}

/// Both halves, as a live session needs them.
pub trait Transcriber: BatchTranscriber + StreamingTranscriber {}

impl<T: BatchTranscriber + StreamingTranscriber> Transcriber for T {}

impl BatchTranscriber for EngineSupervisor {
    fn transcribe(&self, pcm: Vec<f32>) -> Result<BatchTranscript, SttError> {
        Self::transcribe(self, pcm)
    }
}

impl StreamingTranscriber for EngineSupervisor {
    fn open_stream(&self, params: StreamParams) -> Result<Box<dyn TranscriptStream>, SttError> {
        Ok(Box::new(self.start_stream(params)?))
    }
}

impl TranscriptStream for StreamHandle {
    fn feed(&mut self, pcm: Vec<f32>) -> Result<StreamDelta, SttError> {
        Self::feed(self, pcm)
    }

    fn finalize(self: Box<Self>) -> Result<Option<StreamDelta>, SttError> {
        Self::finalize(*self)
    }
}
