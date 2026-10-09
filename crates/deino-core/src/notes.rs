//! Notes: the user's own notes for a meeting ("My notes", NOT-01).
//!
//! The notepad edits a TipTap document ([`deino_doc::Doc`]); [`NoteService`]
//! saves it whole on every autosave into `meeting_notes` (`notes_json`, plus
//! the derived Readable Markdown in `notes_markdown`), so notes typed during
//! a recording are on disk within a second and survive a crash. It also
//! stores pasted and dropped images in the meeting's folder
//! ([`NoteImages`]), creates `scheduled` meetings for notes written before a
//! recording ([`NoteService::create_scheduled`]), and moves a draft's notes
//! into the meeting a recording saved ([`NoteService::adopt`]).
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`MeetingNotes`], [`SavedNotes`], [`NoteImage`], [`ImageKind`], [`ImageName`], limits |
//! | `error.rs` | [`NotesError`] |
//! | `repo.rs` | [`NotesRepo`] and [`SqliteNotesRepo`] (SQL only) |
//! | `images.rs` | [`NoteImages`] and [`FsNoteImages`] |
//! | `events.rs` | [`NotesEvents`] |
//! | `service.rs`, `adopt.rs` | [`NoteService`] |
//! | `fake.rs` | in-memory ports for tests |

mod adopt;
mod error;
mod events;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod images;
mod model;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;
#[cfg(test)]
mod service_tests;

pub use error::NotesError;
pub use events::NotesEvents;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeNotesRepo, MemoryNoteImages, RecordedNotesEvents};
pub use images::{FsNoteImages, NoteImages};
pub use model::{
    IMAGES_FOLDER, ImageKind, ImageName, MAX_DOC_BYTES, MAX_IMAGE_BYTES, MeetingNotes, NoteImage,
    SavedNotes, StoredNotes,
};
pub use repo::{NotesRepo, SqliteNotesRepo};
pub use service::{NotePorts, NoteService};
