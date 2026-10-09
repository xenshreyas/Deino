//! The example's event port: domain moments the service announces.
//!
//! The service never knows who listens. The app implements this trait with a
//! typed Tauri event; tests use `RecordedExampleEvents`. Cache freshness does
//! not depend on it: `DbChanged` already invalidates queries when
//! `example_notes` changes. Events are for moments the UI reacts to, such as
//! highlighting the note that just arrived.

use super::ExampleNote;

/// Receives the example feature's domain events.
pub trait ExampleEvents: Send + Sync {
    /// A note was saved.
    fn note_added(&self, note: &ExampleNote);
}
