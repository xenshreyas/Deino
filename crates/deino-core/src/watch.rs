//! Watched folders (CAP-11): recordings dropped into a folder become
//! meetings on their own.
//!
//! ```text
//! WatchScheduler ─every 2 s─▶ WatchService::scan
//!   for each folder: WatchFs::scan ─▶ StabilityTracker (unchanged for 10 s?)
//!                    ─▶ WatchRepo::handled (the ledger: path, size, mtime)
//!                    ─▶ FileImporter (ImportService) ─▶ WatchRepo::record
//!                    ─▶ WatchFs::move_to_processed (when the folder asks)
//! ```
//!
//! - [`WatchService`]: the folder list (add, remove, "Move to processed"),
//!   the failure list (retry, dismiss) and one scan of every folder.
//! - [`StabilityTracker`]: a file is imported once its size and
//!   modification time have not changed for [`STABLE_FOR`], so a recording
//!   still being written or copied is never read half-done.
//! - [`WatchRepo`] / [`SqliteWatchRepo`]: `watched_folders` and the
//!   `watched_files` ledger, which makes a file import once, across
//!   restarts; files already in a folder when it is added are left alone.
//! - [`WatchFs`] / [`LocalWatchFs`]: listing a folder's media files and
//!   moving one into `Processed/`.
//! - [`WatchScheduler`]: the loop the app spawns once.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod fs;
mod model;
mod repo;
mod schedule;
mod service;
mod stability;

pub use error::WatchError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeImporter, FakeWatchFs, FakeWatchRepo};
pub use fs::{FolderPicker, LocalWatchFs, PROCESSED_FOLDER, WatchFs};
pub use model::{HandledFile, Sighting, WatchFailure, WatchedFolder};
pub use repo::{SqliteWatchRepo, WatchRepo};
pub use schedule::{SCAN_INTERVAL, WatchNudge, WatchScheduler};
pub use service::{WatchDeps, WatchService};
pub use stability::{STABLE_FOR, StabilityTracker};
