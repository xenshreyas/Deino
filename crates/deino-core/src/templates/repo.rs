//! The template storage ports and their SQLite adapter (SQL only).
//!
//! [`TemplateRepo`] is what the library service needs: read everything,
//! save and delete the user's own templates. [`BuiltinStore`]
//! (`builtin_store.rs`) is what `BuiltinSync` needs. A template's row and
//! its sections are always written in one transaction.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{SectionFormat, TemplateId};
use sqlx::types::Json;
use sqlx::{Sqlite, Transaction};

use super::{AutoRule, Template, TemplateContent, TemplateSection};

/// Where templates are stored.
#[async_trait]
pub trait TemplateRepo: Send + Sync {
    /// Every template, built-in and the user's, by name (case-insensitive).
    async fn list(&self) -> Result<Vec<Template>, DbError>;

    /// The template with `id`.
    async fn get(&self, id: &TemplateId) -> Result<Option<Template>, DbError>;

    /// Saves a new template of the user's. Fails if the ID is taken.
    async fn insert(&self, template: &Template) -> Result<(), DbError>;

    /// Replaces the content of one of the user's templates. Returns `false`
    /// when no user template has the ID; built-ins are never changed here.
    async fn update(&self, template: &Template) -> Result<bool, DbError>;

    /// Deletes one of the user's templates (meetings using it go back to
    /// automatic selection). Returns `false` when none has the ID.
    async fn delete(&self, id: &TemplateId) -> Result<bool, DbError>;
}

/// [`TemplateRepo`], [`BuiltinStore`](super::BuiltinStore) and
/// [`MeetingFactsReader`](super::MeetingFactsReader) over SQLite.
#[derive(Clone, Debug)]
pub struct SqliteTemplateRepo {
    pub(super) database: Database,
}

impl SqliteTemplateRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl TemplateRepo for SqliteTemplateRepo {
    async fn list(&self) -> Result<Vec<Template>, DbError> {
        let rows = sqlx::query_file_as!(TemplateRow, "src/templates/queries/list.sql")
            .fetch_all(self.database.reader())
            .await?;
        let sections = sqlx::query_file_as!(SectionRow, "src/templates/queries/sections.sql")
            .fetch_all(self.database.reader())
            .await?;
        let mut by_template: HashMap<TemplateId, Vec<TemplateSection>> = HashMap::new();
        for row in sections {
            by_template
                .entry(row.template_id.clone())
                .or_default()
                .push(row.into_section());
        }
        Ok(rows
            .into_iter()
            .map(|row| {
                let sections = by_template.remove(&row.id).unwrap_or_default();
                row.into_template(sections)
            })
            .collect())
    }

    async fn get(&self, id: &TemplateId) -> Result<Option<Template>, DbError> {
        let row = sqlx::query_file_as!(TemplateRow, "src/templates/queries/template.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        let Some(row) = row else { return Ok(None) };
        let sections =
            sqlx::query_file_as!(SectionRow, "src/templates/queries/sections_of.sql", id)
                .fetch_all(self.database.reader())
                .await?;
        Ok(Some(row.into_template(
            sections.into_iter().map(SectionRow::into_section).collect(),
        )))
    }

    async fn insert(&self, template: &Template) -> Result<(), DbError> {
        let content = &template.content;
        let rules = Json(&content.auto_rules);
        let mut transaction = self.database.writer().begin().await?;
        sqlx::query!(
            "INSERT INTO templates (id, name, description, meeting_context, auto_rules_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            template.id,
            content.name,
            content.description,
            content.meeting_context,
            rules,
            template.created_at,
            template.updated_at,
        )
        .execute(&mut *transaction)
        .await?;
        replace_sections(&mut transaction, &template.id, &content.sections).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn update(&self, template: &Template) -> Result<bool, DbError> {
        let content = &template.content;
        let rules = Json(&content.auto_rules);
        let mut transaction = self.database.writer().begin().await?;
        let updated = sqlx::query!(
            "UPDATE templates SET name = ?1, description = ?2, meeting_context = ?3, auto_rules_json = ?4, updated_at = ?5
             WHERE id = ?6 AND builtin_version IS NULL",
            content.name,
            content.description,
            content.meeting_context,
            rules,
            template.updated_at,
            template.id,
        )
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() == 0 {
            return Ok(false);
        }
        replace_sections(&mut transaction, &template.id, &content.sections).await?;
        transaction.commit().await?;
        Ok(true)
    }

    async fn delete(&self, id: &TemplateId) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "DELETE FROM templates WHERE id = ?1 AND builtin_version IS NULL",
            id
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// Replaces a template's sections, in order, inside `transaction`.
pub(super) async fn replace_sections(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &TemplateId,
    sections: &[TemplateSection],
) -> Result<(), DbError> {
    sqlx::query!("DELETE FROM template_sections WHERE template_id = ?1", id)
        .execute(&mut **transaction)
        .await?;
    for (position, section) in sections.iter().enumerate() {
        let position = i64::try_from(position).unwrap_or(i64::MAX);
        sqlx::query!(
            "INSERT INTO template_sections (template_id, position, title, instructions, format)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            id,
            position,
            section.title,
            section.instructions,
            section.format,
        )
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

/// One `templates` row.
pub(super) struct TemplateRow {
    pub(super) id: TemplateId,
    name: String,
    description: String,
    meeting_context: String,
    auto_rules: Json<Vec<AutoRule>>,
    pub(super) builtin_version: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TemplateRow {
    fn into_template(self, sections: Vec<TemplateSection>) -> Template {
        Template {
            id: self.id,
            is_builtin: self.builtin_version.is_some(),
            content: TemplateContent {
                name: self.name,
                description: self.description,
                meeting_context: self.meeting_context,
                sections,
                auto_rules: self.auto_rules.0,
            },
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// One `template_sections` row.
struct SectionRow {
    template_id: TemplateId,
    title: String,
    instructions: String,
    format: SectionFormat,
}

impl SectionRow {
    fn into_section(self) -> TemplateSection {
        TemplateSection {
            title: self.title,
            instructions: self.instructions,
            format: self.format,
        }
    }
}
