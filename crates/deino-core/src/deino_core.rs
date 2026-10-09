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
//! to copy; [`meetings`] is the first real feature built from it.

#[cfg(debug_assertions)]
pub mod _example;
pub mod meetings;
