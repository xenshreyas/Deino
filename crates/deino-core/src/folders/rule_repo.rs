//! The auto-add rule port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{FolderId, FolderRuleId, FolderRuleKind, SeriesId, TagId};

use super::{FolderRule, FolderRuleEntry, NewFolderRule, SqliteFolderRepo};

/// Where folders' auto-add rules are stored.
#[async_trait]
pub trait FolderRuleRepo: Send + Sync {
    /// Stores the rule. Returns false, changing nothing, when the folder
    /// already has the same rule. Fails if the folder or the rule's tag does
    /// not exist (a foreign-key violation).
    async fn add_rule(&self, rule: &NewFolderRule) -> Result<bool, DbError>;

    /// The folder's rules in the order they were added, series rules with
    /// their series' latest meeting title.
    async fn rules_of(&self, folder: &FolderId) -> Result<Vec<FolderRuleEntry>, DbError>;

    /// Every rule, folder by folder, without series titles.
    async fn rules(&self) -> Result<Vec<FolderRuleEntry>, DbError>;

    /// Deletes the rule.
    async fn remove_rule(&self, id: &FolderRuleId) -> Result<bool, DbError>;

    /// Deletes the folder's rule for `series`, if it has one.
    async fn remove_series_rule(
        &self,
        folder: &FolderId,
        series: &SeriesId,
    ) -> Result<bool, DbError>;
}

#[async_trait]
impl FolderRuleRepo for SqliteFolderRepo {
    async fn add_rule(&self, new: &NewFolderRule) -> Result<bool, DbError> {
        let (series_id, tag_id, domain) = columns(&new.rule);
        let kind = new.rule.kind();
        let result = sqlx::query!(
            "INSERT INTO folder_rules (id, folder_id, kind, series_id, tag_id, domain, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT DO NOTHING",
            new.id,
            new.folder_id,
            kind,
            series_id,
            tag_id,
            domain,
            new.created_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn rules_of(&self, folder: &FolderId) -> Result<Vec<FolderRuleEntry>, DbError> {
        let rows = sqlx::query_file_as!(RuleRow, "src/folders/queries/rules_of.sql", folder)
            .fetch_all(self.database.reader())
            .await?;
        Ok(entries(rows))
    }

    async fn rules(&self) -> Result<Vec<FolderRuleEntry>, DbError> {
        let rows = sqlx::query_file_as!(RuleRow, "src/folders/queries/rules.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(entries(rows))
    }

    async fn remove_rule(&self, id: &FolderRuleId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM folder_rules WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn remove_series_rule(
        &self,
        folder: &FolderId,
        series: &SeriesId,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "DELETE FROM folder_rules WHERE folder_id = ?1 AND series_id = ?2",
            folder,
            series,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// The `series_id`, `tag_id` and `domain` columns of `rule`; exactly one is set.
fn columns(rule: &FolderRule) -> (Option<&SeriesId>, Option<&TagId>, Option<&str>) {
    match rule {
        FolderRule::Series { series_id } => (Some(series_id), None, None),
        FolderRule::Tag { tag_id } => (None, Some(tag_id), None),
        FolderRule::AttendeeDomain { domain } => (None, None, Some(domain)),
    }
}

/// One `folder_rules` row as the queries read it.
struct RuleRow {
    id: FolderRuleId,
    folder_id: FolderId,
    kind: FolderRuleKind,
    series_id: Option<SeriesId>,
    tag_id: Option<TagId>,
    domain: Option<String>,
    created_at: DateTime<Utc>,
    series_title: Option<String>,
}

impl RuleRow {
    /// The rule this row stores. The table's CHECK constraints set exactly the column
    /// of the row's kind, so `None` means a row written outside Deino.
    fn rule(&self) -> Option<FolderRule> {
        match self.kind {
            FolderRuleKind::Series => self
                .series_id
                .clone()
                .map(|series_id| FolderRule::Series { series_id }),
            FolderRuleKind::Tag => self.tag_id.map(|tag_id| FolderRule::Tag { tag_id }),
            FolderRuleKind::AttendeeDomain => self
                .domain
                .clone()
                .map(|domain| FolderRule::AttendeeDomain { domain }),
        }
    }
}

/// The rows as entries, skipping (and logging) any the schema should have refused.
fn entries(rows: Vec<RuleRow>) -> Vec<FolderRuleEntry> {
    rows.into_iter()
        .filter_map(|row| {
            let Some(rule) = row.rule() else {
                log::warn!(target: "folders", "skipping malformed folder rule {}", row.id);
                return None;
            };
            Some(FolderRuleEntry {
                id: row.id,
                folder_id: row.folder_id,
                rule,
                created_at: row.created_at,
                series_title: row.series_title,
            })
        })
        .collect()
}
