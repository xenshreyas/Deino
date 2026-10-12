//! [`InsightRepo`]: the stored definitions, and [`SqliteInsightRepo`], the
//! one SQLite adapter behind every insights port.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{ConditionGroups, InsightApplyMode, InsightId};
use sqlx::types::Json;

use super::{ApplyWhen, Cardinality, InsightDef, InsightField};

/// Reads and writes insight definitions.
#[async_trait]
pub trait InsightRepo: Send + Sync {
    /// Every definition, oldest first.
    async fn list(&self) -> Result<Vec<InsightDef>, DbError>;

    /// The definition with `id`.
    async fn get(&self, id: &InsightId) -> Result<Option<InsightDef>, DbError>;

    /// Stores a new definition; a name in use is a unique violation.
    async fn create(&self, def: &InsightDef) -> Result<(), DbError>;

    /// Replaces a definition; `false` when it does not exist.
    async fn update(&self, def: &InsightDef) -> Result<bool, DbError>;

    /// Deletes a definition and its results; `false` when it did not exist.
    async fn delete(&self, id: &InsightId) -> Result<bool, DbError>;
}

/// Every insights port over Deino's SQLite database.
#[derive(Clone)]
pub struct SqliteInsightRepo {
    pub(super) database: Database,
}

impl SqliteInsightRepo {
    /// The repository over `database`.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// A row of `insight_defs`.
struct DefRow {
    id: InsightId,
    name: String,
    description: String,
    fields: Json<Vec<InsightField>>,
    max_answers: Option<i64>,
    apply_mode: InsightApplyMode,
    conditions: Json<ConditionGroups>,
    revision: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl DefRow {
    fn into_def(self) -> InsightDef {
        let cardinality = match self.max_answers.and_then(|count| u8::try_from(count).ok()) {
            Some(count) => Cardinality::Fixed { count },
            None => Cardinality::Auto,
        };
        let apply_when = match self.apply_mode {
            InsightApplyMode::Automatic => ApplyWhen::Automatic {
                conditions: self.conditions.0,
            },
            InsightApplyMode::Manual => ApplyWhen::Manual,
        };
        InsightDef {
            id: self.id,
            name: self.name,
            description: self.description,
            fields: self.fields.0,
            cardinality,
            apply_when,
            revision: u32::try_from(self.revision).unwrap_or(1),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// The columns a definition is written as.
struct DefColumns<'a> {
    fields: Json<&'a Vec<InsightField>>,
    max_answers: Option<i64>,
    apply_mode: InsightApplyMode,
    conditions: Json<ConditionGroups>,
    revision: i64,
}

impl<'a> DefColumns<'a> {
    fn of(def: &'a InsightDef) -> Self {
        let (apply_mode, conditions) = match &def.apply_when {
            ApplyWhen::Automatic { conditions } => {
                (InsightApplyMode::Automatic, conditions.clone())
            }
            ApplyWhen::Manual => (InsightApplyMode::Manual, ConditionGroups::every_meeting()),
        };
        Self {
            fields: Json(&def.fields),
            max_answers: match def.cardinality {
                Cardinality::Auto => None,
                Cardinality::Fixed { count } => Some(i64::from(count)),
            },
            apply_mode,
            conditions: Json(conditions),
            revision: i64::from(def.revision),
        }
    }
}

#[async_trait]
impl InsightRepo for SqliteInsightRepo {
    async fn list(&self) -> Result<Vec<InsightDef>, DbError> {
        let rows = sqlx::query_file_as!(DefRow, "src/insights/queries/defs.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows.into_iter().map(DefRow::into_def).collect())
    }

    async fn get(&self, id: &InsightId) -> Result<Option<InsightDef>, DbError> {
        let row = sqlx::query_file_as!(DefRow, "src/insights/queries/def.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(row.map(DefRow::into_def))
    }

    async fn create(&self, def: &InsightDef) -> Result<(), DbError> {
        let columns = DefColumns::of(def);
        sqlx::query!(
            "INSERT INTO insight_defs (id, name, description, fields_json, max_answers,
                 apply_mode, conditions_json, revision, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            def.id,
            def.name,
            def.description,
            columns.fields,
            columns.max_answers,
            columns.apply_mode,
            columns.conditions,
            columns.revision,
            def.created_at,
            def.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn update(&self, def: &InsightDef) -> Result<bool, DbError> {
        let columns = DefColumns::of(def);
        let result = sqlx::query!(
            "UPDATE insight_defs SET name = ?1, description = ?2, fields_json = ?3,
                 max_answers = ?4, apply_mode = ?5, conditions_json = ?6, revision = ?7,
                 updated_at = ?8
             WHERE id = ?9",
            def.name,
            def.description,
            columns.fields,
            columns.max_answers,
            columns.apply_mode,
            columns.conditions,
            columns.revision,
            def.updated_at,
            def.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &InsightId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM insight_defs WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
