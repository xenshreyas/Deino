//! Fakes of the ports, for tests here and in other crates (feature
//! `test-support`).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deino_domain::{CallApp, MeetingId, TagSpan};

use crate::error::{AxError, SinkError};
use crate::ports::{
    AccessibilityPermission, Pid, Platform, Position, RecordingClock, RunningApps, TagSink,
    WindowReader,
};
use crate::tree::{AxNode, WindowSummary};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What a fake process shows: its windows, or the error reading them.
type Shown = Result<Vec<AxNode>, AxError>;

/// One fake Mac: a permission switch, running apps and their windows.
#[derive(Debug, Default)]
pub struct FakeMac {
    granted: AtomicBool,
    apps: Mutex<Vec<(String, Pid)>>,
    windows: Mutex<Vec<(Pid, Shown)>>,
    prepared: Mutex<Vec<(CallApp, Pid)>>,
}

impl FakeMac {
    /// A Mac with Accessibility granted and nothing running.
    pub fn granted() -> Arc<Self> {
        let mac = Arc::new(Self::default());
        mac.set_granted(true);
        mac
    }

    /// The ports over this fake.
    pub fn platform(self: &Arc<Self>) -> Platform {
        Platform {
            permission: self.clone(),
            apps: self.clone(),
            windows: self.clone(),
        }
    }

    /// Grants or revokes Accessibility.
    pub fn set_granted(&self, granted: bool) {
        self.granted.store(granted, Ordering::SeqCst);
    }

    /// Runs a process under `bundle_id`.
    pub fn run(&self, bundle_id: &str, pid: Pid) {
        lock(&self.apps).push((bundle_id.to_owned(), pid));
    }

    /// Shows `windows` for `pid` from now on (full trees, as dumped).
    pub fn show(&self, pid: Pid, windows: Shown) {
        let mut shown = lock(&self.windows);
        shown.retain(|(owner, _)| *owner != pid);
        shown.push((pid, windows));
    }

    /// Every `(app, pid)` readied so far.
    pub fn prepared(&self) -> Vec<(CallApp, Pid)> {
        lock(&self.prepared).clone()
    }
}

impl AccessibilityPermission for FakeMac {
    fn granted(&self) -> bool {
        self.granted.load(Ordering::SeqCst)
    }
}

impl RunningApps for FakeMac {
    fn pids(&self, bundle_ids: &[&str]) -> Vec<Pid> {
        lock(&self.apps)
            .iter()
            .filter(|(bundle, _)| bundle_ids.contains(&bundle.as_str()))
            .map(|(_, pid)| *pid)
            .collect()
    }
}

impl WindowReader for FakeMac {
    fn prepare(&self, app: CallApp, pid: Pid) {
        lock(&self.prepared).push((app, pid));
    }

    fn windows(
        &self,
        pid: Pid,
        pick: &dyn Fn(&WindowSummary) -> bool,
    ) -> Result<Vec<AxNode>, AxError> {
        let shown = lock(&self.windows);
        let Some((_, windows)) = shown.iter().find(|(owner, _)| *owner == pid) else {
            return Ok(Vec::new());
        };
        let windows = windows.clone()?;
        Ok(windows
            .into_iter()
            .filter(|window| pick(&window.summary()))
            .collect())
    }
}

/// A recording clock that plays a script of positions, then reports
/// [`Position::Over`].
#[derive(Debug, Default)]
pub struct ScriptedClock {
    script: Mutex<VecDeque<Position>>,
}

impl ScriptedClock {
    /// A clock that will report `positions` in order.
    pub fn new(positions: impl IntoIterator<Item = Position>) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(positions.into_iter().collect()),
        })
    }
}

impl RecordingClock for ScriptedClock {
    fn position(&self, _meeting: &MeetingId) -> Position {
        lock(&self.script).pop_front().unwrap_or(Position::Over)
    }
}

/// Keeps every saved span; can be told to fail.
#[derive(Debug, Default)]
pub struct MemorySink {
    saved: Mutex<Vec<(MeetingId, TagSpan)>>,
    failing: AtomicBool,
}

impl MemorySink {
    /// Every span saved, in order.
    pub fn spans(&self) -> Vec<TagSpan> {
        lock(&self.saved)
            .iter()
            .map(|(_, span)| span.clone())
            .collect()
    }

    /// Makes saves fail (or succeed again).
    pub fn set_failing(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }
}

impl TagSink for MemorySink {
    fn save(&self, meeting: &MeetingId, spans: &[TagSpan]) -> Result<(), SinkError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(SinkError::new(std::io::Error::other("the disk is full")));
        }
        let mut saved = lock(&self.saved);
        saved.extend(spans.iter().map(|span| (meeting.clone(), span.clone())));
        Ok(())
    }
}
