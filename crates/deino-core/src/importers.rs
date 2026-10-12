//! Importers from other note-takers (CAP-12, C §3.10): Granola, Circleback,
//! Fireflies, Fathom and folders of JSON or Markdown files.
//!
//! | File | Role |
//! |---|---|
//! | `source.rs` | the [`Importer`] port: one adapter per source pages through its meetings and fetches each one as a [`TranscriptImport`](crate::transcript_import::TranscriptImport) (the adapters live in `deino-importers`) |
//! | `ledger.rs` | [`ImportLedger`]: the `imported_meetings` de-duplication ledger, by source and source ID |
//! | `runner.rs` | [`ImportRunner`]: the preview count, then the run, writing through DEV-03's `TranscriptImporter` (the one import path) and skipping what the ledger has |
//! | `jobs.rs` | [`ImportJobs`]: one background job at a time, its progress and cancellation |
//! | `wiring.rs` | [`ImportRunner::sqlite`]: the runner over the SQLite repositories |
//! | `model.rs` | [`ImportPreview`], [`ImportProgress`] and the [`ImportEvents`] port |
//!
//! Imported meetings are ordinary `ready` meetings: their transcript and
//! notes reach the keyword index through the search triggers, the importer
//! schedules the `index` stage for embeddings, so search and the Ask panel
//! find them. Nothing here logs meeting content or keys.

mod error;
mod jobs;
mod ledger;
mod model;
mod runner;
mod source;
#[cfg(any(test, feature = "test-support"))]
mod testing;
mod wiring;

pub use error::ImporterError;
pub use jobs::ImportJobs;
pub use ledger::{ImportLedger, SqliteImportLedger};
pub use model::{ImportEvents, ImportPhase, ImportPreview, ImportProgress, NoImportEvents};
pub use runner::{ImportRunner, MAX_PAGES};
pub use source::{Importer, SourceItem, SourcePage};
#[cfg(any(test, feature = "test-support"))]
pub use testing::ImportWorld;
