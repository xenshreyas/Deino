//! The notes event port: the moments other windows react to.
//!
//! Queries stay fresh through `DbChanged` on `meeting_notes`; this event
//! exists so a second notepad on the same meeting (the record panel's
//! "Open notes" window, CAP-10) can tell its own save from someone else's.

use super::SavedNotes;

/// Receives the notes feature's domain events.
pub trait NotesEvents: Send + Sync {
    /// A meeting's notes were saved.
    fn saved(&self, saved: &SavedNotes);
}
