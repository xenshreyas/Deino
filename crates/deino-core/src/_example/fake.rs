//! In-memory ports for tests. Fakes live next to the trait they implement,
//! behind the `test-support` feature, and honour the trait's full contract.

use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use deino_db::DbError;

use super::{ExampleEvents, ExampleNote, ExampleNoteRepo};

/// Keeps notes in a `Vec`, listed newest first like the SQLite adapter.
#[derive(Debug, Default)]
pub struct FakeExampleNoteRepo {
    notes: Mutex<Vec<ExampleNote>>,
}

#[async_trait]
impl ExampleNoteRepo for FakeExampleNoteRepo {
    async fn list(&self) -> Result<Vec<ExampleNote>, DbError> {
        let mut notes = lock(&self.notes).clone();
        notes.sort_by(|a, b| (&b.created_at, &b.id).cmp(&(&a.created_at, &a.id)));
        Ok(notes)
    }

    async fn insert(&self, note: &ExampleNote) -> Result<(), DbError> {
        lock(&self.notes).push(note.clone());
        Ok(())
    }
}

/// Records every event so tests can assert on what was announced.
#[derive(Debug, Default)]
pub struct RecordedExampleEvents {
    added: Mutex<Vec<ExampleNote>>,
}

impl RecordedExampleEvents {
    /// The notes announced through `note_added`, in order.
    pub fn added(&self) -> Vec<ExampleNote> {
        lock(&self.added).clone()
    }
}

impl ExampleEvents for RecordedExampleEvents {
    fn note_added(&self, note: &ExampleNote) {
        lock(&self.added).push(note.clone());
    }
}

/// Locks a fake's state. A panic in another test thread cannot leave a
/// `Vec` half-updated, so a poisoned lock is still safe to use.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
