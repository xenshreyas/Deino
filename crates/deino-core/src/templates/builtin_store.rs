//! The [`BuiltinStore`] port, through which `BuiltinSync` writes the
//! catalogue, and the SQLite adapters of it and of `MeetingFactsReader`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{Email, MeetingId, TemplateId};
use sqlx::types::Json;

use super::repo::replace_sections;
use super::{MeetingFacts, MeetingFactsReader, SqliteTemplateRepo, Template};

/// Where built-in templates are stored, by catalogue version.
#[async_trait]
pub trait BuiltinStore: Send + Sync {
    /// Every stored built-in's ID and the catalogue version it was written from.
    async fn builtin_versions(&self) -> Result<Vec<(TemplateId, u32)>, DbError>;

    /// Inserts or overwrites the built-in `template` as catalogue `version`,
    /// sections included, in one transaction. Never overwrites a user template.
    async fn save_builtin(&self, template: &Template, version: u32) -> Result<(), DbError>;

    /// Deletes a built-in the catalogue no longer has.
    async fn remove_builtin(&self, id: &TemplateId) -> Result<bool, DbError>;
}

#[async_trait]
impl BuiltinStore for SqliteTemplateRepo {
    async fn builtin_versions(&self) -> Result<Vec<(TemplateId, u32)>, DbError> {
        let rows = sqlx::query!(
            r#"SELECT id AS "id: TemplateId", builtin_version AS "version!: i64"
               FROM templates WHERE builtin_version IS NOT NULL"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.id, u32::try_from(row.version).unwrap_or_default()))
            .collect())
    }

    async fn save_builtin(&self, template: &Template, version: u32) -> Result<(), DbError> {
        let content = &template.content;
        let rules = Json(&content.auto_rules);
        let version = i64::from(version);
        let mut transaction = self.database.writer().begin().await?;
        let saved = sqlx::query!(
            "INSERT INTO templates (id, name, description, meeting_context, auto_rules_json, builtin_version, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name, description = excluded.description,
                 meeting_context = excluded.meeting_context, auto_rules_json = excluded.auto_rules_json,
                 builtin_version = excluded.builtin_version, updated_at = excluded.updated_at
             WHERE templates.builtin_version IS NOT NULL",
            template.id,
            content.name,
            content.description,
            content.meeting_context,
            rules,
            version,
            template.updated_at,
        )
        .execute(&mut *transaction)
        .await?;
        if saved.rows_affected() > 0 {
            replace_sections(&mut transaction, &template.id, &content.sections).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn remove_builtin(&self, id: &TemplateId) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "DELETE FROM templates WHERE id = ?1 AND builtin_version IS NOT NULL",
            id
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait]
impl MeetingFactsReader for SqliteTemplateRepo {
    async fn facts(&self, meeting: &MeetingId) -> Result<Option<MeetingFacts>, DbError> {
        let row = sqlx::query!(
            r#"SELECT meetings.id AS "id!: MeetingId", meetings.title,
                   meetings.template_id AS "template_id: TemplateId",
                   meetings.started_at AS "started_at: DateTime<Utc>",
                   coalesce(calendar_events.description, '') AS "description!: String"
               FROM meetings
               LEFT JOIN calendar_events ON calendar_events.id = meetings.calendar_event_id
               WHERE meetings.id = ?1"#,
            meeting,
        )
        .fetch_optional(self.database.reader())
        .await?;
        let Some(row) = row else { return Ok(None) };
        let emails: Vec<Option<Email>> =
            sqlx::query_file_scalar!("src/templates/queries/attendee_emails.sql", meeting)
                .fetch_all(self.database.reader())
                .await?;
        Ok(Some(MeetingFacts {
            id: row.id,
            title: row.title,
            template_id: row.template_id,
            started_at: row.started_at,
            description: row.description,
            attendee_count: emails.len(),
            attendee_emails: emails.into_iter().flatten().collect(),
        }))
    }
}
