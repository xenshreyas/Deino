//! The engine a worker process drives: the only code that touches a model.
//!
//! [`Engine`] is what `worker::run` serves requests against. The real one is
//! [`TranscribeCppEngine`] (transcribe-cpp); tests and the fake worker binary
//! use `ScriptedEngine`, which turns loud audio into numbered words so every
//! lifecycle (and every injected fault) is deterministic.
//!
//! A stream borrows its engine (transcribe-cpp's `Stream<'_>` borrows the
//! session), so it is a separate [`EngineStream`] that lives only while the
//! worker serves that stream.

#[cfg(any(test, feature = "test-support"))]
pub mod scripted;
mod transcribe_cpp;

pub use transcribe_cpp::TranscribeCppEngine;

use crate::protocol::{
    BatchParams, BatchTranscript, LoadRequest, LoadedInfo, RemoteError, StreamDelta, StreamParams,
};

/// A speech-to-text engine, owned by one worker process.
pub trait Engine {
    /// The stream type, borrowing the engine while it is open.
    type Stream<'a>: EngineStream
    where
        Self: 'a;

    /// Registers the compute backends: the CPU only, or every available one.
    /// Called once, before the first request.
    fn init(&mut self, cpu_only: bool) -> Result<(), RemoteError>;

    /// Opens the GPU so its shaders compile now rather than on first use.
    fn warm_gpu(&mut self);

    /// Loads a model, dropping any loaded one first.
    fn load(&mut self, request: &LoadRequest) -> Result<LoadedInfo, RemoteError>;

    /// Transcribes `pcm` (16 kHz mono) in one call.
    fn run(&mut self, pcm: &[f32], params: &BatchParams) -> Result<BatchTranscript, RemoteError>;

    /// Opens a stream on the loaded model.
    fn open_stream(&mut self, params: &StreamParams) -> Result<Self::Stream<'_>, RemoteError>;
}

/// An open stream: feed audio in order, then finalize once.
pub trait EngineStream {
    /// Feeds more 16 kHz mono audio and returns what it committed.
    fn feed(&mut self, pcm: &[f32]) -> Result<StreamDelta, RemoteError>;

    /// Flushes the stream and returns its last tokens.
    fn finalize(&mut self) -> Result<StreamDelta, RemoteError>;
}
