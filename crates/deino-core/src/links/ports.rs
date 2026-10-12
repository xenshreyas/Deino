//! The ports [`super::LinkService`] works over; the app crate implements
//! them over the recorder, the app lock and the main window.

use async_trait::async_trait;
use deino_domain::ErrorCode;

/// Why the recorder refused or failed, as its error maps to IPC: a
/// `conflict` when already recording or idle, `unavailable` when the
/// microphone or the model cannot start.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RecorderError {
    /// The error's class.
    pub code: ErrorCode,
    /// The recorder's user-safe message.
    pub message: String,
}

/// The live recorder: CAP-10's start and stop path, which the menu bar,
/// the record panel and the UI use too.
#[async_trait]
pub trait Recorder: Send + Sync {
    /// True while a recording runs (paused included).
    fn is_recording(&self) -> bool;
    /// Starts recording a new meeting with the default title.
    async fn start(&self) -> Result<(), RecorderError>;
    /// Stops the recording and hands its meeting to the pipeline.
    async fn stop(&self) -> Result<(), RecorderError>;
}

/// The app lock (PRV-02), as far as links need it.
pub trait LockProbe: Send + Sync {
    /// True while the library is hidden behind the lock.
    fn is_locked(&self) -> bool;
}

/// The main window.
pub trait Shell: Send + Sync {
    /// Brings the window forward (it shows the lock screen while locked).
    fn focus(&self);
    /// Tells the UI a navigation is waiting to be taken.
    fn navigation_waiting(&self);
}
