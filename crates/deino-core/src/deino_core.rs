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
//! DEV-02/04 added [`query`], the read side the `deino` CLI and its MCP
//! server answer from.
//!
//! TRN-05 added [`vocabulary`]: the user's terms and replacement rules, and
//! the corrections they and the known names make to transcripts. CAP-08
//! added [`final_transcription`]: transcribing after the meeting when live
//! transcription is off, and the optional high-accuracy re-pass.
//!
//! NOT-04 and NOT-05 added [`templates`] (the note templates, built-ins
//! seeded from a versioned catalogue, auto selection) and [`profile`] (the
//! user's profile as AI context, the "Me" person, internal vs external).
//!
//! FND-11 added [`permissions`]: the macOS permissions onboarding asks for,
//! over a port the app implements; and [`sample`], the bundled sample
//! meeting onboarding offers.
//!
//! TRN-01 added [`transcript`]: a meeting's transcript with its speakers'
//! display names, as the transcript view reads it; TRN-02 its edits (edit,
//! delete, find and replace) and TRN-03 [`player`], the recording files the
//! meeting player plays and downloads.
//!
//! ORG-04 added [`entities`]: the rules over people and companies (upserts,
//! merges, pages, AI summaries, company logos).
//!
//! NOT-09 added [`redaction`]: passwords, keys, card numbers, SSNs and
//! one-time codes replaced by `[redacted]` before notes and exports.
//!
//! CAP-05 added [`speaker_tags`]: the names a meeting app's window showed
//! for its active speaker, stored while recording and used by
//! `identify_speakers` before voices are matched.
//!
//! ORG-09 added [`stats`]: meeting and LLM usage stats computed on read,
//! and the year in review.
//!
//! CAP-11 added [`import`] (an audio or video file becomes a meeting the
//! pipeline transcribes) and [`watch`] (watched folders that import new
//! recordings on their own, once each).
//! PRV-02 added [`privacy`] (bringing the database file in line with the
//! encryption setting before it opens) and [`lock`] (the app lock).

#[cfg(debug_assertions)]
pub mod _example;
pub mod action_items;
pub mod automations;
pub mod briefs;
pub mod calendar;
pub mod cli_setup;
pub mod clips;
pub mod comments;
pub mod companies;
pub mod diarization;
pub mod enhance;
pub mod entities;
pub mod export;
mod f32_blob;
pub mod final_transcription;
#[cfg(test)]
mod fixture_tests;
pub mod folders;
pub mod import;
pub mod insights;
pub mod lock;
pub mod meetings;
pub mod models;
pub mod notes;
pub mod people;
pub mod permissions;
pub mod player;
pub mod privacy;
pub mod profile;
pub mod query;
pub mod redaction;
pub mod retention;
pub mod sample;
pub mod search;
pub mod segments;
pub mod settings;
pub mod speaker_tags;
pub mod speakers;
pub mod stats;
pub mod tags;
pub mod templates;
#[cfg(any(test, feature = "test-support"))]
mod testing;
pub mod transcript;
pub mod vocabulary;
pub mod voiceprints;
pub mod watch;
