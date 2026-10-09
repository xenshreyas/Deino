//! [`FilingService`]: which meetings sit in which folders, and the rules
//! that file future meetings automatically.
//!
//! A meeting can sit in many folders. Filing a meeting from a recurring
//! series offers to collect the whole series (Granola's "Auto-add future
//! meetings?"); taking one out stops collecting it.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{Clock, FolderId, FolderRuleId, MeetingId, SeriesId};

use super::{
    FiledMeeting, Folder, FolderError, FolderPorts, FolderRepo, FolderRule, FolderRuleEntry,
    FolderRuleRepo, MeetingFacts, MeetingFactsReader, MeetingFolderRepo, NewFolderRule,
    UnfiledMeeting,
};
use crate::meetings::Meeting;

/// Files meetings into folders and keeps folders' auto-add rules.
pub struct FilingService {
    folders: Arc<dyn FolderRepo>,
    filing: Arc<dyn MeetingFolderRepo>,
    rules: Arc<dyn FolderRuleRepo>,
    facts: Arc<dyn MeetingFactsReader>,
    clock: Arc<dyn Clock>,
}

impl FilingService {
    /// Creates the service over its ports.
    pub fn new(ports: &FolderPorts) -> Self {
        Self {
            folders: Arc::clone(&ports.folders),
            filing: Arc::clone(&ports.filing),
            rules: Arc::clone(&ports.rules),
            facts: Arc::clone(&ports.facts),
            clock: Arc::clone(&ports.clock),
        }
    }

    /// The meetings in the folder, newest first.
    pub async fn meetings(&self, folder: &FolderId) -> Result<Vec<Meeting>, FolderError> {
        self.require_folder(folder).await?;
        Ok(self.filing.meetings_in(folder).await?)
    }

    /// The meetings in no folder, newest first: the "Unfiled" smart folder.
    pub async fn unfiled(&self) -> Result<Vec<Meeting>, FolderError> {
        Ok(self.filing.unfiled().await?)
    }

    /// The folders the meeting sits in, by name.
    pub async fn folders_of(&self, meeting: &MeetingId) -> Result<Vec<Folder>, FolderError> {
        Ok(self.filing.folders_of(meeting).await?)
    }

    /// Files the meeting into the folder (again: no change) and offers its
    /// series when the folder does not collect it yet.
    pub async fn add_meeting(
        &self,
        folder: &FolderId,
        meeting: &MeetingId,
    ) -> Result<FiledMeeting, FolderError> {
        self.require_folder(folder).await?;
        let facts = self.require_facts(meeting).await?;
        self.filing
            .file_meeting(meeting, folder, self.clock.now())
            .await?;
        log::info!(target: "folders", "filed meeting {meeting} in folder {folder}");
        let series_offer = match facts.series_id {
            Some(series) if !self.collects(folder, &series).await? => Some(series),
            _ => None,
        };
        Ok(FiledMeeting { series_offer })
    }

    /// Takes the meeting out of the folder; the folder also stops collecting
    /// the meeting's series, if it did.
    pub async fn remove_meeting(
        &self,
        folder: &FolderId,
        meeting: &MeetingId,
    ) -> Result<UnfiledMeeting, FolderError> {
        self.require_folder(folder).await?;
        self.filing.unfile_meeting(meeting, folder).await?;
        let series = self
            .facts
            .facts(meeting)
            .await?
            .and_then(|facts| facts.series_id);
        let series_rule_removed = match series {
            Some(series) => self.rules.remove_series_rule(folder, &series).await?,
            None => false,
        };
        log::info!(target: "folders", "unfiled meeting {meeting} from folder {folder}");
        Ok(UnfiledMeeting {
            series_rule_removed,
        })
    }

    /// The folder's auto-add rules, oldest first.
    pub async fn rules(&self, folder: &FolderId) -> Result<Vec<FolderRuleEntry>, FolderError> {
        self.require_folder(folder).await?;
        Ok(self.rules.rules_of(folder).await?)
    }

    /// Adds an auto-add rule to the folder; adding a rule it already has
    /// returns the existing one.
    pub async fn add_rule(
        &self,
        folder: &FolderId,
        rule: FolderRule,
    ) -> Result<FolderRuleEntry, FolderError> {
        self.require_folder(folder).await?;
        let rule = rule.normalized()?;
        let new = NewFolderRule {
            id: FolderRuleId::generate(self.clock.as_ref()),
            folder_id: *folder,
            rule,
            created_at: self.clock.now(),
        };
        self.rules
            .add_rule(&new)
            .await
            .map_err(|error| missing_tag(error, &new.rule))?;
        let stored = self.rules.rules_of(folder).await?;
        let entry = stored.into_iter().find(|entry| entry.rule == new.rule);
        log::info!(target: "folders", "added a {} rule to folder {folder}", new.rule.kind());
        entry.ok_or(FolderError::NotFound(*folder))
    }

    /// Deletes an auto-add rule. Meetings it filed stay where they are.
    pub async fn remove_rule(&self, id: &FolderRuleId) -> Result<(), FolderError> {
        if self.rules.remove_rule(id).await? {
            Ok(())
        } else {
            Err(FolderError::RuleNotFound(*id))
        }
    }

    async fn require_folder(&self, folder: &FolderId) -> Result<(), FolderError> {
        match self.folders.folder(folder).await? {
            Some(_) => Ok(()),
            None => Err(FolderError::NotFound(*folder)),
        }
    }

    async fn require_facts(&self, meeting: &MeetingId) -> Result<MeetingFacts, FolderError> {
        self.facts
            .facts(meeting)
            .await?
            .ok_or_else(|| FolderError::MeetingNotFound(meeting.clone()))
    }

    /// True when the folder already has a rule for `series`.
    async fn collects(&self, folder: &FolderId, series: &SeriesId) -> Result<bool, FolderError> {
        let wanted = FolderRule::Series {
            series_id: series.clone(),
        };
        let rules = self.rules.rules_of(folder).await?;
        Ok(rules.iter().any(|entry| entry.rule == wanted))
    }
}

/// A foreign-key failure while adding `rule`: its tag is gone (the folder
/// was checked first).
fn missing_tag(error: DbError, rule: &FolderRule) -> FolderError {
    match rule {
        FolderRule::Tag { tag_id } if error.is_foreign_key_violation() => {
            FolderError::TagNotFound(*tag_id)
        }
        _ => FolderError::Storage(error),
    }
}
