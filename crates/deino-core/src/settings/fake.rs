//! In-memory ports for tests, behind `test-support`.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;

use super::{Settings, SettingsError, SettingsEvents, SettingsStore};

/// Keeps the "saved" settings in memory and can be told to fail saves.
#[derive(Debug, Default)]
pub struct FakeSettingsStore {
    saved: Mutex<Option<Settings>>,
    fail_saves: AtomicBool,
}

impl FakeSettingsStore {
    /// A store that already holds `saved`.
    pub fn with(saved: Option<Settings>) -> Self {
        Self {
            saved: Mutex::new(saved),
            fail_saves: AtomicBool::new(false),
        }
    }

    /// The settings saved last, if any.
    pub fn saved(&self) -> Option<Settings> {
        lock(&self.saved).clone()
    }

    /// Makes every later save fail, like a full or read-only disk.
    pub fn fail_saves(&self) {
        self.fail_saves.store(true, Ordering::SeqCst);
    }
}

#[async_trait]
impl SettingsStore for FakeSettingsStore {
    async fn load(&self) -> Result<Option<Settings>, SettingsError> {
        Ok(self.saved().map(Settings::upgraded))
    }

    async fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        if self.fail_saves.load(Ordering::SeqCst) {
            return Err(SettingsError::Write(io::Error::other("disk full")));
        }
        *lock(&self.saved) = Some(settings.clone());
        Ok(())
    }
}

/// Records every announced change, in order.
#[derive(Debug, Default)]
pub struct RecordedSettingsEvents {
    changes: Mutex<Vec<Settings>>,
}

impl RecordedSettingsEvents {
    /// The settings announced so far.
    pub fn changes(&self) -> Vec<Settings> {
        lock(&self.changes).clone()
    }
}

impl SettingsEvents for RecordedSettingsEvents {
    fn changed(&self, settings: &Settings) {
        lock(&self.changes).push(settings.clone());
    }
}

/// A panic in another test thread cannot leave the state half-updated.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
