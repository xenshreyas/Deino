//! The meeting player's files (TRN-03).
//!
//! The UI plays a meeting's recording through Tauri's asset protocol, which
//! the app scopes to the recordings folder (`<app data>/recordings`).
//! [`PlayerService::media`] lists the files a meeting's row names (ADR
//! 0026: the mixed `audio.mp4`, `mic.m4a` "Me", `system.m4a` "Them"; a
//! screen recording when CAP-09 adds one) that exist inside that folder, so
//! the UI only asks for what it may load. [`PlayerService::download`] copies
//! one where the user chooses in the save dialog; no command takes a path
//! from the UI.

mod files;
mod model;
mod service;
#[cfg(test)]
mod service_tests;

pub use files::{FsMediaFiles, MediaFiles};
pub use model::{DownloadOutcome, MediaFile, MediaSource, MeetingMedia};
pub use service::{PlayerDeps, PlayerError, PlayerService};
