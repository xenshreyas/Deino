//! Clips (TRN-04, ADR 0048): named ranges of a meeting's recording, each cut
//! into its own file.
//!
//! [`ClipService::create`] cuts `[start, end)` of the meeting's screen
//! recording (`.mp4`) or mixed audio (`.m4a`) into
//! `<recording folder>/clips/<name>.<ext>` through the [`ClipCutter`] port
//! (the app wires `deino_audio::clip`: `AVAssetExportSession`, Passthrough,
//! no re-encode), with a safe file name that is unique in that folder
//! ([`naming`]). The clip lives inside the recordings folder, so the asset
//! protocol can serve it and Finder can only be asked to reveal files there.
//! Clips of a meeting in the trash are never listed.
//!
//! | File | Layer | Owns |
//! |---|---|---|
//! | `model.rs` | domain | [`Clip`], [`NewClip`], [`ClipMedia`] |
//! | `naming.rs` | pure | safe, unique file names |
//! | `ports.rs` | ports | [`ClipSources`] (implemented by `PlayerService`), [`ClipCutter`], [`ClipFiles`], [`Finder`] |
//! | `files.rs` | adapter | [`FsClipFiles`]: the files, confined to the recordings folder |
//! | `repo.rs` | storage | [`ClipRepo`], [`SqliteClipRepo`] |
//! | `service.rs` | rules | [`ClipService`] |
//! | `fake.rs` | test doubles | in-memory ports behind `test-support` |

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod files;
mod model;
pub mod naming;
mod ports;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;
#[cfg(test)]
mod service_tests;

pub use error::ClipError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeClipCutter, FakeClipFiles, FakeClipSources, FakeFinder};
pub use files::FsClipFiles;
pub use model::{Clip, ClipMedia, ClipRename, ClipRow, ClipSource, CutRequest, NewClip};
pub use ports::{ClipCutter, ClipFiles, ClipSources, CutFailed, Finder};
pub use repo::{ClipRepo, SqliteClipRepo};
pub use service::{CLIPS_FOLDER, ClipDeps, ClipService};
