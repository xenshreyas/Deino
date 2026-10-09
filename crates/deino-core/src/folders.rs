//! Folders: nested collections of meetings (ORG-05).
//!
//! - [`FolderService`]: the tree's rules: create, rename, icon,
//!   description, cycle-safe moves, deletion that keeps every meeting, and
//!   the tree in display order with meeting counts.
//! - [`FilingService`]: meetings in folders (a meeting can sit in many), the
//!   "Unfiled" smart folder, the "Auto-add future meetings in this series?"
//!   offer, and each folder's auto-add rules.
//! - [`FolderRule`] and [`folders_matching`]: the typed rules (a recurring
//!   series, a tag, an attendee's email domain) and the pure rule engine.
//! - [`AutoFileService`]: the [`AutoFiler`] the `auto_folder` pipeline
//!   stage (order 75) calls.
//!
//! Ports: [`FolderRepo`] (the tree; the only writer of `parent_id`, so it
//! can refuse cycles), [`MeetingFolderRepo`] (filings), [`FolderRuleRepo`]
//! (`folder_rules`) and [`MeetingFactsReader`] (what rules match on). One
//! [`SqliteFolderRepo`] implements all four; `FakeFolderRepo` stands in for
//! tests.

mod auto_file;
mod error;
mod facts_repo;
#[cfg(any(test, feature = "test-support"))]
mod fake;
#[cfg(any(test, feature = "test-support"))]
mod fake_rules;
mod filing;
#[cfg(test)]
mod filing_tests;
#[cfg(test)]
mod folder_tests;
mod input;
mod meeting_folder_repo;
mod model;
mod repo;
mod rule;
mod rule_repo;
#[cfg(test)]
mod rule_repo_tests;
mod service;
#[cfg(test)]
mod service_tests;
mod tree;

use std::sync::Arc;

use deino_domain::Clock;

pub use auto_file::{AutoFileService, AutoFiled, AutoFiler};
pub use error::FolderError;
pub use facts_repo::MeetingFactsReader;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeFolderRepo;
pub use filing::FilingService;
pub use input::{
    FolderDescription, FolderIcon, FolderName, MAX_FOLDER_DESCRIPTION_CHARS, MAX_FOLDER_ICON_CHARS,
    MAX_FOLDER_NAME_CHARS,
};
pub use meeting_folder_repo::MeetingFolderRepo;
pub use model::{FiledMeeting, Folder, FolderMove, FolderTree, FolderTreeEntry, UnfiledMeeting};
pub use repo::{FolderRepo, SqliteFolderRepo};
pub use rule::{FolderRule, FolderRuleEntry, MeetingFacts, NewFolderRule, folders_matching};
pub use rule_repo::FolderRuleRepo;
pub use service::{FolderService, NewFolder};

/// The ports every folder service is built from. The app passes one
/// [`SqliteFolderRepo`] for all four repositories; tests pass a fake.
#[derive(Clone)]
pub struct FolderPorts {
    /// The folder tree.
    pub folders: Arc<dyn FolderRepo>,
    /// Which meetings sit in which folders.
    pub filing: Arc<dyn MeetingFolderRepo>,
    /// The folders' auto-add rules.
    pub rules: Arc<dyn FolderRuleRepo>,
    /// What rules match on.
    pub facts: Arc<dyn MeetingFactsReader>,
    /// The clock that stamps changes.
    pub clock: Arc<dyn Clock>,
}

impl FolderPorts {
    /// Every port served by one repository that implements them all.
    pub fn over<R>(repo: Arc<R>, clock: Arc<dyn Clock>) -> Self
    where
        R: FolderRepo + MeetingFolderRepo + FolderRuleRepo + MeetingFactsReader + 'static,
    {
        Self {
            folders: repo.clone(),
            filing: repo.clone(),
            rules: repo.clone(),
            facts: repo,
            clock,
        }
    }
}
