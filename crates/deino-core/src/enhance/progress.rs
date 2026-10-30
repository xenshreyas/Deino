//! What a notes run reports while it writes (streamed notes,
//! ADR 2026-10-30-streamed-notes): the [`NotesProgress`] port, the
//! [`NotesDraft`] it receives, and how a run ended ([`NotesRunEnd`]).
//!
//! A draft holds only finished lines (see `partial.rs`), checked like the
//! saved notes (`assemble::preview`), as the same TipTap document the saved
//! version will be, so the UI shows it with the notes' own styles and the
//! saved version replaces it without a visible change. It carries no
//! evidence; nothing here is ever logged.

use std::sync::{Arc, OnceLock};
#[cfg(any(test, feature = "test-support"))]
use std::sync::{Mutex, PoisonError};

use deino_doc::Doc;
use deino_domain::MeetingId;
use serde::Serialize;

/// What a run is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NotesPhase {
    /// One call is reading the transcript and writing the notes.
    Reading,
    /// A transcript too long for one call is noted in parts first.
    Parts {
        /// Parts noted so far.
        done: u32,
        /// Parts in all.
        total: u32,
    },
    /// The parts' notes are being merged into the notes.
    Merging,
    /// An answer could not be used; the model is asked again.
    Retrying,
    /// The notes are written and being saved.
    Saving,
}

/// The notes as far as they are written.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NotesDraft {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// What the run is doing.
    pub phase: NotesPhase,
    /// The finished lines so far; empty before the first.
    pub doc: Doc,
    /// How many lines `doc` holds (headings, blocks, details).
    pub lines: u32,
    /// True while `doc` belongs to an answer being replaced (a retry, or a
    /// transcript noted in parts after one call failed): shown dimmed until
    /// the new answer's first line replaces it.
    pub stale: bool,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NotesRunEnd {
    /// The notes were saved as this version.
    Written {
        /// The new version.
        version: u32,
        /// How many lines the user typed, every one kept word for word.
        kept: u32,
    },
    /// The user stopped it; nothing was saved.
    Stopped,
    /// It failed; nothing was saved.
    Failed,
}

/// Receives a run's drafts and its end. Must not block: it is called
/// between the pieces of the model's answer.
pub trait NotesProgress: Send + Sync {
    /// The run got further.
    fn draft(&self, draft: &NotesDraft);
    /// The run ended (only for a run that reported a draft).
    fn ended(&self, meeting: &MeetingId, end: NotesRunEnd);
}

/// Reports nowhere: the default until the app binds its adapter.
#[derive(Debug, Default)]
pub struct NoProgress;

impl NotesProgress for NoProgress {
    fn draft(&self, _draft: &NotesDraft) {}

    fn ended(&self, _meeting: &MeetingId, _end: NotesRunEnd) {}
}

/// A port bound after the service is built: for wiring that exists before
/// the app handle the real adapter needs. Drops reports until bound.
#[derive(Default)]
pub struct LateProgress {
    inner: OnceLock<Arc<dyn NotesProgress>>,
}

impl LateProgress {
    /// Sends every later report to `progress`; a second bind is ignored.
    pub fn bind(&self, progress: Arc<dyn NotesProgress>) {
        if self.inner.set(progress).is_err() {
            log::debug!(target: "enhance", "the notes progress was already bound");
        }
    }
}

impl NotesProgress for LateProgress {
    fn draft(&self, draft: &NotesDraft) {
        if let Some(inner) = self.inner.get() {
            inner.draft(draft);
        }
    }

    fn ended(&self, meeting: &MeetingId, end: NotesRunEnd) {
        if let Some(inner) = self.inner.get() {
            inner.ended(meeting, end);
        }
    }
}

/// One report, as [`RecordedProgress`] keeps it.
#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Debug, PartialEq)]
pub enum Reported {
    /// A draft.
    Draft(NotesDraft),
    /// An end.
    Ended(MeetingId, NotesRunEnd),
}

/// Keeps every report, for tests.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct RecordedProgress {
    reports: Mutex<Vec<Reported>>,
}

#[cfg(any(test, feature = "test-support"))]
impl RecordedProgress {
    /// Every report so far, oldest first.
    pub fn reports(&self) -> Vec<Reported> {
        self.lock().clone()
    }

    /// Every draft so far, oldest first.
    pub fn drafts(&self) -> Vec<NotesDraft> {
        self.lock()
            .iter()
            .filter_map(|report| match report {
                Reported::Draft(draft) => Some(draft.clone()),
                Reported::Ended(..) => None,
            })
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Reported>> {
        self.reports.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(any(test, feature = "test-support"))]
impl NotesProgress for RecordedProgress {
    fn draft(&self, draft: &NotesDraft) {
        self.lock().push(Reported::Draft(draft.clone()));
    }

    fn ended(&self, meeting: &MeetingId, end: NotesRunEnd) {
        self.lock().push(Reported::Ended(meeting.clone(), end));
    }
}
