//! The `enhance` plugin's typed event and the adapter that emits it.
//!
//! While a meeting's notes are written (here or by the pipeline), the
//! service reports each new finished line and how the run ended through
//! its `NotesProgress` port; [`TauriNotesProgress`] implements the port by
//! broadcasting [`NotesRun`] (`plugin:enhance:notes-run`), because the
//! pipeline's runs have no request to answer over a channel. The `draft`
//! command gives a page opened mid-run the latest draft; a missed event
//! costs a moment of staleness, and the saved version always arrives
//! through `DbChanged` on `enhanced_notes`.

use deino_core::enhance::{NotesDraft, NotesProgress, NotesRunEnd};
use deino_domain::MeetingId;
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// A meeting's notes got further while being written, or the run ended.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct NotesRun {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// What happened.
    pub update: NotesRunUpdate,
}

/// What a [`NotesRun`] reports.
#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NotesRunUpdate {
    /// The notes as far as they are written.
    Draft {
        /// The draft.
        draft: NotesDraft,
    },
    /// The run ended.
    Ended {
        /// How.
        end: NotesRunEnd,
    },
}

/// Emits notes runs to the UI.
pub struct TauriNotesProgress<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriNotesProgress<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    fn emit(&self, run: &NotesRun) {
        if let Err(error) = run.emit(&self.app) {
            log::warn!(target: "enhance", "could not emit notes-run for meeting {}: {error}", run.meeting_id);
        }
    }
}

impl<R: Runtime> NotesProgress for TauriNotesProgress<R> {
    fn draft(&self, draft: &NotesDraft) {
        self.emit(&NotesRun {
            meeting_id: draft.meeting_id.clone(),
            update: NotesRunUpdate::Draft {
                draft: draft.clone(),
            },
        });
    }

    fn ended(&self, meeting: &MeetingId, end: NotesRunEnd) {
        self.emit(&NotesRun {
            meeting_id: meeting.clone(),
            update: NotesRunUpdate::Ended { end },
        });
    }
}
