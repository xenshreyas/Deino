//! In-memory ports for [`super::LinkService`] tests.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use async_trait::async_trait;
use deino_domain::ErrorCode;

use super::ports::{LockProbe, Recorder, RecorderError, Shell};

/// A recorder that records what it was asked, like the real one refusing
/// to start twice or to stop when idle.
#[derive(Debug, Default)]
pub struct FakeRecorder {
    recording: AtomicBool,
    calls: Mutex<Vec<&'static str>>,
}

impl FakeRecorder {
    /// A recorder that is recording already.
    pub fn recording() -> Self {
        let recorder = Self::default();
        recorder.recording.store(true, Ordering::SeqCst);
        recorder
    }

    /// Every call, in order: `"start"` or `"stop"`.
    pub fn calls(&self) -> Vec<&'static str> {
        self.calls
            .lock()
            .map(|calls| calls.clone())
            .unwrap_or_default()
    }

    fn call(&self, name: &'static str, want: bool) -> Result<(), RecorderError> {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(name);
        }
        if self.recording.swap(want, Ordering::SeqCst) == want {
            return Err(RecorderError {
                code: ErrorCode::Conflict,
                message: format!("cannot {name} now"),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl Recorder for FakeRecorder {
    fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    async fn start(&self) -> Result<(), RecorderError> {
        self.call("start", true)
    }

    async fn stop(&self) -> Result<(), RecorderError> {
        self.call("stop", false)
    }
}

/// A lock the test turns on and off.
#[derive(Debug, Default)]
pub struct FakeLock(AtomicBool);

impl FakeLock {
    /// Locks or unlocks.
    pub fn set(&self, locked: bool) {
        self.0.store(locked, Ordering::SeqCst);
    }
}

impl LockProbe for FakeLock {
    fn is_locked(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A window that counts how often it was focused and told.
#[derive(Debug, Default)]
pub struct FakeShell {
    focused: AtomicUsize,
    told: AtomicUsize,
}

impl FakeShell {
    /// How often the window was brought forward.
    pub fn focused(&self) -> usize {
        self.focused.load(Ordering::SeqCst)
    }

    /// How often the UI was told a navigation waits.
    pub fn told(&self) -> usize {
        self.told.load(Ordering::SeqCst)
    }
}

impl Shell for FakeShell {
    fn focus(&self) {
        self.focused.fetch_add(1, Ordering::SeqCst);
    }

    fn navigation_waiting(&self) {
        self.told.fetch_add(1, Ordering::SeqCst);
    }
}
