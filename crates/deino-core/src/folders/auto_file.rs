//! Auto-filing: the `auto_folder` pipeline stage's work (order 75).
//!
//! After processing (and after `auto_tag`, so tag rules see the AI's tags),
//! a meeting is filed into every folder whose rules match it. Filing is
//! idempotent, so a re-run changes nothing.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{Clock, FolderId, MeetingId};

use super::{
    FolderError, FolderPorts, FolderRuleRepo, MeetingFactsReader, MeetingFolderRepo,
    folders_matching,
};

/// How auto-filing a meeting went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutoFiled {
    /// The meeting was filed into these folders (some may already have held it).
    Filed(Vec<FolderId>),
    /// No folder has an auto-add rule.
    NoRules,
    /// Rules exist, but none matches the meeting.
    NoMatch,
    /// The meeting is gone or in the trash.
    MeetingGone,
}

/// Files a meeting by the folders' auto-add rules.
#[async_trait]
pub trait AutoFiler: Send + Sync {
    /// Files `meeting` into every folder whose rules match it.
    async fn auto_file(&self, meeting: &MeetingId) -> Result<AutoFiled, FolderError>;
}

/// [`AutoFiler`] over the folder ports and the rule engine.
pub struct AutoFileService {
    filing: Arc<dyn MeetingFolderRepo>,
    rules: Arc<dyn FolderRuleRepo>,
    facts: Arc<dyn MeetingFactsReader>,
    clock: Arc<dyn Clock>,
}

impl AutoFileService {
    /// Creates the service over its ports.
    pub fn new(ports: &FolderPorts) -> Self {
        Self {
            filing: Arc::clone(&ports.filing),
            rules: Arc::clone(&ports.rules),
            facts: Arc::clone(&ports.facts),
            clock: Arc::clone(&ports.clock),
        }
    }
}

#[async_trait]
impl AutoFiler for AutoFileService {
    async fn auto_file(&self, meeting: &MeetingId) -> Result<AutoFiled, FolderError> {
        let rules = self.rules.rules().await?;
        if rules.is_empty() {
            return Ok(AutoFiled::NoRules);
        }
        let Some(facts) = self.facts.facts(meeting).await? else {
            return Ok(AutoFiled::MeetingGone);
        };
        let folders = folders_matching(&rules, &facts);
        if folders.is_empty() {
            return Ok(AutoFiled::NoMatch);
        }
        let now = self.clock.now();
        for folder in &folders {
            self.filing.file_meeting(meeting, folder, now).await?;
        }
        log::info!(target: "folders", "auto-filed meeting {meeting} into {} folders", folders.len());
        Ok(AutoFiled::Filed(folders))
    }
}
