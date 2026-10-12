//! The sections of [`Settings`](super::Settings), one file each.
//!
//! A section file declares its types and registers itself with
//! [`register_section!`](super::register_section). `build.rs` re-exports
//! every file here by a glob (the generated `pub use` lines below) and turns
//! the registrations into the fields of `Settings`. The only shared line a
//! new section adds is its `mod` line, alphabetical, in this file.

mod ai;
mod api;
mod audio;
mod automations;
mod briefs;
mod calendar;
mod consent;
mod detection;
mod export;
mod integrations;
mod network;
mod notifications;
mod onboarding;
mod organize;
mod personal;
mod profile;
mod retention;
mod screen;
mod shortcuts;
mod speakers;
mod templates;
mod transcription;
mod ui;

include!(concat!(env!("OUT_DIR"), "/settings_sections.rs"));
