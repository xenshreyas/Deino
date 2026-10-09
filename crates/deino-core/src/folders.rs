//! Folders: nested collections of meetings.
//!
//! [`FolderRepo`] stores the folder tree and is the only writer of a
//! folder's parent, refusing any move that would make a cycle;
//! [`MeetingFolderRepo`] files meetings into folders (a meeting can sit in
//! many). [`SqliteFolderRepo`] implements both and `FakeFolderRepo` stands
//! in for tests. ORG-05 adds the rules (auto-add rules, the "Unfiled" smart
//! folder) and the sidebar tree.

#[cfg(any(test, feature = "test-support"))]
mod fake;
#[cfg(test)]
mod folder_tests;
mod meeting_folder_repo;
mod model;
mod repo;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeFolderRepo;
pub use meeting_folder_repo::MeetingFolderRepo;
pub use model::{Folder, FolderMove};
pub use repo::{FolderRepo, SqliteFolderRepo};
