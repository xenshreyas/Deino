//! The reference feature: copy this module to start a new one.
//!
//! A tiny "notes" feature that shows every layer of a Deino feature with
//! nothing else in the way (PARITY_PLAN §3A):
//!
//! | File | Layer | Rule |
//! |---|---|---|
//! | `model.rs` | types | Validate on construction; invalid values cannot exist. |
//! | `error.rs` | errors | One `thiserror` enum implementing `ErrorClass`. |
//! | `repo.rs` | storage port + SQLite adapter | SQL only, `sqlx::query*!` macros. |
//! | `events.rs` | event port | Domain moments; the app maps them to IPC events. |
//! | `service.rs` | business logic | Ports in, no SQL, no Tauri, time from `Clock`. |
//! | `fake.rs` | test doubles | In-memory ports behind `test-support`. |
//!
//! The app crate exposes it as the inline plugin `features/_example.rs`, and
//! the UI uses it from `frontend/src/features/_example/`.
//!
//! It is compiled into debug builds only and keeps its notes in a private
//! in-memory database with its own [`MIGRATOR`], so release builds and the
//! user's `deino.sqlite` never see it. A real feature puts its migration in
//! `crates/deino-db/migrations/` and its repository on the app's `Database`.

mod error;
mod events;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod repo;
mod service;

pub use error::ExampleError;
pub use events::ExampleEvents;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeExampleNoteRepo, RecordedExampleEvents};
pub use model::{ExampleNote, MAX_NOTE_CHARS, NoteText};
pub use repo::{ExampleNoteRepo, SqliteExampleNoteRepo};
pub use service::ExampleService;

/// The example's own schema, applied to its in-memory database.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("src/_example/migrations");
