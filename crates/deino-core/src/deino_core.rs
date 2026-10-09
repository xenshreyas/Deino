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
//! [`settings`] holds every user preference.
//!
//! FND-07 added one module per aggregate of the core schema, each with its
//! ports, a SQLite repository and a fake, so feature cards add services and
//! commands on top: [`segments`] (transcript and speakers), [`people`]
//! (people and attendance), [`companies`], [`calendar`], [`folders`],
//! [`tags`] and [`action_items`], plus the lifecycle and trash ports of
//! [`meetings`].

#[cfg(debug_assertions)]
pub mod _example;
pub mod action_items;
pub mod calendar;
pub mod companies;
#[cfg(test)]
mod fixture_tests;
pub mod folders;
pub mod meetings;
pub mod models;
pub mod people;
pub mod segments;
pub mod settings;
pub mod tags;
#[cfg(any(test, feature = "test-support"))]
mod testing;
