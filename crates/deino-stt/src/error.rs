//! What can go wrong in speech-to-text, as the app sees it.
//!
//! Messages are user-safe: they name the failure, never transcript text.

use std::io;
use std::time::Duration;

use crate::protocol::RemoteError;

/// A speech-to-text failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SttError {
    /// The request needs a loaded model.
    #[error("no transcription model is loaded")]
    NotLoaded,
    /// The model cannot stream (Whisper, for example).
    #[error("the transcription model cannot stream")]
    StreamingUnsupported,
    /// The open stream is gone (its worker crashed, hung or failed): the
    /// audio fed since its last committed segment must be transcribed again.
    #[error("the live transcription stream was lost")]
    StreamLost,
    /// `cancel` stopped the work.
    #[error("the transcription was cancelled")]
    Cancelled,
    /// The worker process crashed or hung and could not be recovered.
    #[error("the transcription worker {0}")]
    WorkerLost(WorkerFault),
    /// The worker process could not be started.
    #[error("could not start the transcription worker")]
    Spawn(#[source] io::Error),
    /// The engine refused or failed the request; the worker is fine.
    #[error("the transcription engine failed: {0}")]
    Engine(#[from] RemoteError),
    /// The supervisor's thread is gone (the app is shutting down).
    #[error("the transcription engine has stopped")]
    Stopped,
}

/// How a worker process was lost.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkerFault {
    /// It exited or crashed; `status` is its exit status and last native line.
    #[error("crashed ({status})")]
    Crashed {
        /// The exit status and the last native output line.
        status: String,
    },
    /// It did not answer within `after` and was killed.
    #[error("did not respond within {after:?}")]
    Hung {
        /// The deadline it missed.
        after: Duration,
    },
}
