//! Importers from other note-takers (CAP-12, C §3.10, G §9.2): one adapter
//! per source behind `deino_core::importers::Importer`, so adding a source
//! is one module here and one line in [`ImportSources::provider`].
//!
//! | Module | Source |
//! |---|---|
//! | [`granola`] | Granola's public API: `GET /v1/notes`, then each note with its transcript |
//! | [`circleback`] | Circleback's API: `GET /meetings`, `/meeting/{id}`, `/meeting/{id}/transcript` |
//! | [`fireflies`] | Fireflies' GraphQL API: `transcripts(limit, skip)` |
//! | [`fathom`] | Fathom's API: `GET /external/v1/meetings` with transcripts, summaries and action items |
//! | [`folder`] | JSON (the local API's import shape) and Markdown files in a folder |
//!
//! Each adapter maps the source's notes, transcript (speakers and times),
//! attendees and action items into a `TranscriptImport`; the core's runner
//! writes it through DEV-03's importer. Requests go through `deino-net`
//! scoped to the source's `ServiceId`; keys live in the Keychain. Nothing
//! here logs keys or meeting content.

pub mod circleback;
mod factory;
pub mod fathom;
pub mod fireflies;
pub mod folder;
pub mod granola;
mod http;
mod mapping;

pub use circleback::CirclebackImporter;
pub use factory::{ImportSources, secret, service};
pub use fathom::FathomImporter;
pub use fireflies::FirefliesImporter;
pub use folder::FolderImporter;
pub use granola::GranolaImporter;
