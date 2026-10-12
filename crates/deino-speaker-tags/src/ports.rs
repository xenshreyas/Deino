//! The ports a watch runs on. `macos/` implements the platform ones; the
//! app implements [`RecordingClock`] over the recorder and [`TagSink`] over
//! the database; `fake` (feature `test-support`) has one of each for tests.

use std::sync::Arc;

use deino_domain::{CallApp, MeetingId, Millis, TagSpan};

use crate::error::{AxError, SinkError};
use crate::tree::{AxNode, WindowSummary};

/// A Unix process id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pid(pub i32);

/// Whether Deino may read other apps' windows.
pub trait AccessibilityPermission: Send + Sync {
    /// Whether Accessibility is granted. Never shows the system prompt.
    fn granted(&self) -> bool;
}

/// Which processes run an app.
pub trait RunningApps: Send + Sync {
    /// The processes running under any of `bundle_ids`.
    fn pids(&self, bundle_ids: &[&str]) -> Vec<Pid>;
}

/// Reads an app's windows through Accessibility.
pub trait WindowReader: Send + Sync {
    /// Readies a process of `app` for reading; called once per process.
    /// Chrome exposes web content to Accessibility only when asked.
    fn prepare(&self, app: CallApp, pid: Pid);

    /// The full trees of `pid`'s windows that `pick` accepts from their
    /// title and address, front to back. Trees are cut at a size budget.
    fn windows(
        &self,
        pid: Pid,
        pick: &dyn Fn(&WindowSummary) -> bool,
    ) -> Result<Vec<AxNode>, AxError>;
}

/// The platform's ports, together.
#[derive(Clone)]
pub struct Platform {
    /// Whether Accessibility is granted.
    pub permission: Arc<dyn AccessibilityPermission>,
    /// Which meeting apps run.
    pub apps: Arc<dyn RunningApps>,
    /// Their windows.
    pub windows: Arc<dyn WindowReader>,
}

/// Where a recording is on its own timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// Recording, this far from its start (pauses skipped).
    At(Millis),
    /// Paused: the timeline stands still.
    Paused,
    /// The recording ended (or another one runs).
    Over,
}

/// The recording's clock.
pub trait RecordingClock: Send + Sync {
    /// Where the recording of `meeting` is now.
    fn position(&self, meeting: &MeetingId) -> Position;
}

/// Where finished spans go.
pub trait TagSink: Send + Sync {
    /// Stores `spans` for `meeting`.
    fn save(&self, meeting: &MeetingId, spans: &[TagSpan]) -> Result<(), SinkError>;
}
