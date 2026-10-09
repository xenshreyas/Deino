//! How one call to a worker failed, before it becomes an [`SttError`].

use std::io;
use std::time::Duration;

use crate::error::{SttError, WorkerFault};
use crate::protocol::RemoteError;

/// A failed worker call.
#[derive(Debug)]
pub(super) enum Failure {
    /// The worker process could not be started.
    Spawn(io::Error),
    /// The engine returned an error; the worker is fine.
    Remote(RemoteError),
    /// The worker exited or crashed: its exit status and last native line.
    Crashed(String),
    /// The worker missed its deadline and was killed.
    Hung(Duration),
    /// `cancel` stopped the call.
    Cancelled,
}

impl Failure {
    /// Whether the worker is gone.
    pub(super) fn worker_lost(&self) -> bool {
        matches!(self, Self::Crashed(_) | Self::Hung(_))
    }

    /// An answer that does not fit the request: a protocol bug, reported as
    /// an engine error so the worker is not blamed.
    pub(super) fn unexpected(what: &str) -> Self {
        Self::Remote(RemoteError::new(
            crate::protocol::RemoteErrorKind::Inference,
            format!("unexpected worker answer to {what}"),
        ))
    }
}

impl From<Failure> for SttError {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Spawn(error) => Self::Spawn(error),
            Failure::Remote(error) => Self::Engine(error),
            Failure::Crashed(status) => Self::WorkerLost(WorkerFault::Crashed { status }),
            Failure::Hung(after) => Self::WorkerLost(WorkerFault::Hung { after }),
            Failure::Cancelled => Self::Cancelled,
        }
    }
}
