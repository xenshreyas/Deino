//! The rule and facts halves of `FakeFolderRepo`.

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{FolderId, FolderRuleId, MeetingId, SeriesId};

use super::{
    FakeFolderRepo, FolderRule, FolderRuleEntry, FolderRuleRepo, MeetingFacts, MeetingFactsReader,
    NewFolderRule,
};
use crate::testing::lock;

fn entry(rule: &NewFolderRule) -> FolderRuleEntry {
    FolderRuleEntry {
        id: rule.id,
        folder_id: rule.folder_id,
        rule: rule.rule.clone(),
        created_at: rule.created_at,
        series_title: None,
    }
}

#[async_trait]
impl FolderRuleRepo for FakeFolderRepo {
    async fn add_rule(&self, new: &NewFolderRule) -> Result<bool, DbError> {
        let mut rules = lock(&self.rules);
        let exists = rules
            .iter()
            .any(|stored| stored.folder_id == new.folder_id && stored.rule == new.rule);
        if !exists {
            rules.push(new.clone());
        }
        Ok(!exists)
    }

    async fn rules_of(&self, folder: &FolderId) -> Result<Vec<FolderRuleEntry>, DbError> {
        let mut rules: Vec<FolderRuleEntry> = lock(&self.rules)
            .iter()
            .filter(|rule| rule.folder_id == *folder)
            .map(entry)
            .collect();
        rules.sort_by_key(|rule| (rule.created_at, rule.id));
        Ok(rules)
    }

    async fn rules(&self) -> Result<Vec<FolderRuleEntry>, DbError> {
        let mut rules: Vec<FolderRuleEntry> = lock(&self.rules).iter().map(entry).collect();
        rules.sort_by_key(|rule| (rule.folder_id, rule.created_at, rule.id));
        Ok(rules)
    }

    async fn remove_rule(&self, id: &FolderRuleId) -> Result<bool, DbError> {
        let mut rules = lock(&self.rules);
        let before = rules.len();
        rules.retain(|rule| rule.id != *id);
        Ok(rules.len() < before)
    }

    async fn remove_series_rule(
        &self,
        folder: &FolderId,
        series: &SeriesId,
    ) -> Result<bool, DbError> {
        let doomed = FolderRule::Series {
            series_id: series.clone(),
        };
        let mut rules = lock(&self.rules);
        let before = rules.len();
        rules.retain(|rule| !(rule.folder_id == *folder && rule.rule == doomed));
        Ok(rules.len() < before)
    }
}

#[async_trait]
impl MeetingFactsReader for FakeFolderRepo {
    async fn facts(&self, meeting: &MeetingId) -> Result<Option<MeetingFacts>, DbError> {
        if !self.knows(meeting) {
            return Ok(None);
        }
        Ok(Some(
            lock(&self.facts).get(meeting).cloned().unwrap_or_default(),
        ))
    }
}
