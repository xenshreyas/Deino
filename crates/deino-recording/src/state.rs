//! The recorder's published state: one [`RecordingSnapshot`] behind a lock,
//! announced to the UI on every change. The service changes phases; a
//! session's health watcher changes the open issues.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::events::{RecordingEvents, RecordingSnapshot};

/// The recorder's state and where its changes go. Cloning shares both.
#[derive(Clone)]
pub(crate) struct StateCell {
    snapshot: Arc<Mutex<RecordingSnapshot>>,
    events: Arc<dyn RecordingEvents>,
}

impl StateCell {
    /// An idle recorder announcing to `events`.
    pub fn new(events: Arc<dyn RecordingEvents>) -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(RecordingSnapshot::idle())),
            events,
        }
    }

    /// The state now.
    pub fn get(&self) -> RecordingSnapshot {
        lock(&self.snapshot).clone()
    }

    /// Changes the state, announces it and returns it. The announcement is
    /// made under the lock, so the UI hears changes in the order they were
    /// made (event adapters never block).
    pub fn update(&self, change: impl FnOnce(&mut RecordingSnapshot)) -> RecordingSnapshot {
        let mut state = lock(&self.snapshot);
        change(&mut state);
        self.events.state(&state);
        state.clone()
    }

    /// Replaces the state and announces it.
    pub fn set(&self, state: RecordingSnapshot) -> RecordingSnapshot {
        self.update(|current| *current = state)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
