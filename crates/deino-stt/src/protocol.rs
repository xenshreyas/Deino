//! The messages between the app and its transcription worker process.
//!
//! Strict request/response: every [`Request`] gets exactly one [`Response`].
//! The types are Deino's own rather than transcribe-cpp's, so the supervisor,
//! the fake engine and the tests never need the native library, and the
//! worker returns only what the app uses (new committed tokens, not whole
//! snapshots). Framing is in [`frame`]. Ported from Handy's
//! `engine_supervisor/protocol.rs` (MIT).

pub mod frame;

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::window::StreamWindow;
use crate::words::TimedToken;

/// What the app asks of the worker. `Run` and `Feed` frames carry PCM.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Request {
    /// The first request to every worker, answered once its compute backends
    /// are registered. With `warm_gpu`, the worker also opens the GPU, which
    /// compiles the Metal shader library on first use (about 15 s).
    Hello {
        /// Open the GPU before answering.
        warm_gpu: bool,
    },
    /// Load a model, replacing any loaded one.
    Load(LoadRequest),
    /// Transcribe the frame's PCM in one call.
    Run(BatchParams),
    /// Open a stream; `Feed`, `Finalize` and `StreamReset` then apply to it.
    StreamBegin(StreamParams),
    /// Feed the frame's PCM to the open stream.
    Feed,
    /// Flush the open stream and close it.
    Finalize,
    /// Abandon the open stream.
    StreamReset,
}

/// The worker's answer to one [`Request`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Response {
    /// Backends are ready.
    Hello,
    /// The model loaded.
    Loaded(LoadedInfo),
    /// The result of a `Run`.
    Transcript(BatchTranscript),
    /// What a `Feed` added.
    Fed(StreamDelta),
    /// What `Finalize` added; the stream is closed.
    Finalized(StreamDelta),
    /// A request with nothing to return succeeded.
    Ok,
    /// The request failed; the worker is fine.
    Error(RemoteError),
}

/// Which compute backend a model should load on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendChoice {
    /// The best available device: Metal on Apple Silicon.
    Auto,
    /// The CPU only.
    Cpu,
}

/// A model to load.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadRequest {
    /// The model file (GGUF, or a legacy whisper.cpp `.bin`).
    pub path: PathBuf,
    /// Where to run it.
    pub backend: BackendChoice,
}

/// What the app needs to know about a loaded model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedInfo {
    /// Model family, such as `parakeet` or `whisper`.
    pub arch: String,
    /// The backend the model bound to, such as `MTL0` or `CPU`.
    pub backend: String,
    /// Whether that backend is a GPU (a crash there may be the GPU's fault).
    pub on_gpu: bool,
    /// Whether the model can stream.
    pub supports_streaming: bool,
    /// The longest audio one `Run` accepts, in ms (0 when unlimited).
    pub max_audio_ms: i64,
}

/// Options for a one-shot transcription.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchParams {}

/// Options for a stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamParams {
    /// The buffered streaming window, or the model's default.
    pub window: Option<StreamWindow>,
}

/// The result of a one-shot transcription; token times are relative to the
/// start of the PCM.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BatchTranscript {
    /// The whole text.
    pub text: String,
    /// Every token, when the model reports timestamps.
    pub tokens: Vec<TimedToken>,
}

/// How far a stream has got, in ms of stream audio.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamProgress {
    /// Audio received since the stream began.
    pub received_ms: i64,
    /// Audio the decoder has finished with.
    pub committed_ms: i64,
    /// Audio still buffered inside the decoder.
    pub buffered_ms: i64,
}

impl StreamProgress {
    /// Audio received but not yet decoded: the engine's backlog.
    pub fn pending_ms(self) -> i64 {
        self.buffered_ms
            .max(self.received_ms - self.committed_ms)
            .max(0)
    }
}

/// What one feed or finalize added to a stream.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamDelta {
    /// Where the stream stands after the call.
    pub progress: StreamProgress,
    /// Tokens newly committed by this call, times relative to the stream start.
    /// Committed tokens are final: they are never revised.
    pub tokens: Vec<TimedToken>,
    /// The uncommitted tail, which may still be rewritten; `None` when unchanged.
    pub tentative: Option<String>,
}

/// A request the worker could not serve.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct RemoteError {
    /// What kind of failure it was.
    pub kind: RemoteErrorKind,
    /// The engine's description; never transcript text.
    pub message: String,
}

impl RemoteError {
    /// An error of `kind` described by `message`.
    pub fn new(kind: RemoteErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// The kinds of [`RemoteError`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteErrorKind {
    /// No compute backend could be registered.
    BackendInit,
    /// The model could not be loaded.
    Load,
    /// The request needs a loaded model.
    NotLoaded,
    /// The request needs an open stream.
    NoStream,
    /// A stream is open; finalize or reset it first.
    StreamActive,
    /// The model cannot serve this request, such as streaming on Whisper.
    Unsupported,
    /// Transcription itself failed.
    Inference,
}

impl fmt::Display for RemoteErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BackendInit => "backend initialization failed",
            Self::Load => "model load failed",
            Self::NotLoaded => "no model is loaded",
            Self::NoStream => "no stream is open",
            Self::StreamActive => "a stream is open",
            Self::Unsupported => "not supported by this model",
            Self::Inference => "transcription failed",
        })
    }
}

/// A short name for a request, for logs and timeouts.
pub fn request_name(request: &Request) -> &'static str {
    match request {
        Request::Hello { .. } => "hello",
        Request::Load(_) => "model load",
        Request::Run(_) => "transcription",
        Request::StreamBegin(_) => "stream begin",
        Request::Feed => "stream feed",
        Request::Finalize => "stream finalize",
        Request::StreamReset => "stream reset",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_audio_is_the_larger_of_buffered_and_undecoded() {
        let progress = StreamProgress {
            received_ms: 3000,
            committed_ms: 2000,
            buffered_ms: 600,
        };
        assert_eq!(progress.pending_ms(), 1000);
        let caught_up = StreamProgress {
            received_ms: 2000,
            committed_ms: 2000,
            buffered_ms: 0,
        };
        assert_eq!(caught_up.pending_ms(), 0);
    }

    #[test]
    fn remote_errors_name_their_kind() {
        let error = RemoteError::new(RemoteErrorKind::Load, "file missing");

        assert_eq!(error.to_string(), "model load failed: file missing");
    }
}
