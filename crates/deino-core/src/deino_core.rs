//! Deino's features as Tauri-free modules.
//!
//! Each feature is a module `<feature>.rs` + `<feature>/` with the same
//! layers (PARITY_PLAN §3A):
//!
//! - `model.rs`: the feature's types, validated on construction;
//! - `error.rs`: a `thiserror` enum implementing `deino_domain::ErrorClass`;
//! - `repo.rs`: a port trait plus its SQLite implementation (SQL only);
//! - `service.rs`: business logic over the ports, with no SQL and no Tauri;
//! - `fake.rs`: an in-memory port implementation behind `test-support`.
//!
//! The app crate wires adapters into services (`composition.rs`) and exposes
//! each service through one inline Tauri plugin. [`_example`] is the template
//! to copy; [`meetings`] is the first real feature built from it, and
//! [`settings`] holds every user preference; [`notes`] keeps the user's own
//! notes (TipTap documents via `deino-doc`) and [`enhance`] turns them and
//! the transcript into the meeting's enhanced notes through the LLM gateway
//! (`deino-llm`, NOT-02).
//!
//! [`diarization`] holds the `diarize` stage's rules (CAP-03) over an engine
//! port that `deino-diarize` implements.
//!
//! FND-07 added one module per aggregate of the core schema, each with its
//! ports, a SQLite repository and a fake, so feature cards add services and
//! commands on top: [`segments`] (transcript and speakers), [`people`]
//! (people and attendance), [`companies`], [`calendar`], [`folders`],
//! [`tags`] and [`action_items`], plus the lifecycle and trash ports of
//! [`meetings`].
//!
//! NOT-04 and NOT-05 added [`templates`] (the note templates, built-ins
//! seeded from a versioned catalogue, auto selection) and [`profile`] (the
//! user's profile as AI context, the "Me" person, internal vs external).
//!
//! TRN-01 added [`transcript`]: a meeting's transcript with its speakers'
//! display names, as the transcript view reads it.

#[cfg(debug_assertions)]
pub mod _example;
pub mod action_items;
pub mod calendar;
pub mod companies;
pub mod diarization;
pub mod enhance;
pub mod export;
mod f32_blob;
#[cfg(test)]
mod fixture_tests;
pub mod folders;
pub mod meetings;
pub mod models;
pub mod notes;
pub mod people;
pub mod profile;
pub mod search;
pub mod segments;
pub mod settings;
pub mod speakers;
pub mod tags;
pub mod templates;
#[cfg(any(test, feature = "test-support"))]
mod testing;
pub mod transcript;
pub mod voiceprints;
