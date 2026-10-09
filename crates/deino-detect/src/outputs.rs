//! The ports [`crate::DetectionService`] acts through: where its events go,
//! how a recording starts, and the user's settings. The app implements them
//! over Tauri and the settings store; tests use the fakes.

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_domain::BundleId;

use crate::error::DetectError;
use crate::event::{Detection, DetectionEvent};
use crate::model::AppIdentity;

/// Receives the typed detection stream: the app shows or hides the prompt
/// and forwards the events to the UI. Must return quickly.
pub trait DetectionSink: Send + Sync {
    /// Publishes one event.
    fn publish(&self, event: DetectionEvent);
}

/// A recording to start for a detected meeting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingRequest {
    /// The meeting.
    pub detection: Detection,
    /// Bring the meeting's notes forward: the user clicked "Take notes".
    /// An auto-start records without taking focus from the call.
    pub open_notes: bool,
}

/// Starts recording (CAP-02 owns the recording itself).
pub trait RecordingStarter: Send + Sync {
    /// Starts recording `request`'s meeting.
    fn start(&self, request: &RecordingRequest) -> Result<(), DetectError>;
}

/// Whether Deino is recording right now.
pub trait RecordingStatus: Send + Sync {
    /// True from the moment recording starts until it has stopped.
    fn is_recording(&self) -> bool;
}

/// The user's detection settings, as detection needs them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DetectionConfig {
    /// Detection is switched on.
    pub enabled: bool,
    /// Apps that never prompt.
    pub ignored: HashSet<BundleId>,
    /// The auto-start countdown, when auto-start is on.
    pub auto_start: Option<TimeDelta>,
}

/// Reads the settings; called on every evaluation, so a change applies on
/// the next tick.
pub trait ConfigSource: Send + Sync {
    /// The current settings.
    fn config(&self) -> DetectionConfig;
}

/// Saves "Always ignore this app".
#[async_trait]
pub trait IgnoreList: Send + Sync {
    /// Adds `app` to the ignore list.
    async fn ignore(&self, app: &AppIdentity) -> Result<(), DetectError>;
}
