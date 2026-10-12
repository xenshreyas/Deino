//! Feature IPC: one inline Tauri plugin per feature (ADR 0002, ADR 0007).
//!
//! Each feature exposes `NAME`, `specta()` (its `tauri_specta::Builder`) and
//! `init()` from `features/<feature>.rs`, with thin commands in
//! `features/<feature>/commands.rs`. **A feature registers itself from its
//! own files** (ADR 2026-10-12, feature self-registration):
//!
//! 1. `features/<feature>.rs`: `register_plugin!(Guarded)` (or `Open`) adds
//!    it to [`registry::PLUGINS`], which `lib.rs` registers behind the app
//!    lock's guard and `bindings.rs` exports to TypeScript;
//! 2. `features/<feature>/commands.rs`: its `collect_commands![..]`, which
//!    `build.rs` reads to generate the plugin's ACL and the capability that
//!    grants it to the `main` window.
//!
//! The only shared line is the `pub mod` below (alphabetical, one per line).
//! A debug-only feature says so in its own file with
//! `#![cfg(debug_assertions)]`. The guard tests in [`acl`] and `bindings.rs`
//! fail when the plugins, their ACL and their bindings drift apart.
//! [`_example`] is the reference feature to copy.

pub mod _example;
pub mod action_items;
pub mod automations;
pub mod briefs;
pub mod calendar;
pub mod clips;
pub mod comments;
pub mod companies;
pub mod db;
pub mod detection;
pub mod dev;
pub mod developer;
pub mod enhance;
pub mod export;
pub mod folders;
pub mod import;
pub mod insights;
pub mod integrations;
pub mod llm;
pub mod meetings;
pub mod network;
pub mod notes;
pub mod onboarding;
pub mod people;
pub mod pipeline;
pub mod player;
pub mod privacy;
pub mod profile;
pub mod recording;
pub mod registry;
pub mod search;
pub mod secrets;
pub mod settings;
pub mod speaker_tags;
pub mod speakers;
pub mod stats;
pub mod system;
pub mod tags;
pub mod templates;
pub mod transcript;
pub mod transcription;
pub mod vocabulary;
pub mod webhooks;

#[cfg(test)]
pub(crate) mod acl;
#[cfg(test)]
mod ipc_tests;
