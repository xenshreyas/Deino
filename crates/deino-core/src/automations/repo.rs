//! [`AutomationRepo`]: the stored automations, and [`SqliteAutomationRepo`],
//! the one SQLite adapter behind every automations port.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{AutomationId, ConditionGroups};
use sqlx::types::Json;

use super::{Action, Automation};

/// Reads and writes automations.
#[async_trait]
pub trait AutomationRepo: Send + Sync {
    /// Every automation, oldest first.
    async fn list(&self) -> Result<Vec<Automation>, DbError>;

    /// The automation with `id`.
    async fn get(&self, id: &AutomationId) -> Result<Option<Automation>, DbError>;

    /// Stores a new automation; a name in use is a unique violation.
    async fn create(&self, automation: &Automation) -> Result<(), DbError>;

    /// Replaces an automation; `false` when it does not exist.
    async fn update(&self, automation: &Automation) -> Result<bool, DbError>;

    /// Deletes an automation and its runs; `false` when it did not exist.
    async fn delete(&self, id: &AutomationId) -> Result<bool, DbError>;
}

/// Every automations port over Deino's SQLite database.
#[derive(Clone)]
pub struct SqliteAutomationRepo {
    pub(super) database: Database,
}

impl SqliteAutomationRepo {
    /// The repository over `database`.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// A row of `automations`.
struct AutomationRow {
    id: AutomationId,
    name: String,
    enabled: bool,
    auto_run: bool,
    stop_on_failure: bool,
    conditions: Json<ConditionGroups>,
    steps: Json<Vec<Action>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<AutomationRow> for Automation {
    fn from(row: AutomationRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            enabled: row.enabled,
            auto_run: row.auto_run,
            stop_on_failure: row.stop_on_failure,
            conditions: row.conditions.0,
            steps: row.steps.0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl AutomationRepo for SqliteAutomationRepo {
    async fn list(&self) -> Result<Vec<Automation>, DbError> {
        let rows = sqlx::query_as!(
            AutomationRow,
            r#"SELECT id AS "id: AutomationId", name, enabled AS "enabled: bool",
                   auto_run AS "auto_run: bool", stop_on_failure AS "stop_on_failure: bool",
                   conditions_json AS "conditions: Json<ConditionGroups>",
                   steps_json AS "steps: Json<Vec<Action>>",
                   created_at AS "created_at: DateTime<Utc>",
                   updated_at AS "updated_at: DateTime<Utc>"
               FROM automations ORDER BY created_at, id"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(Automation::from).collect())
    }

    async fn get(&self, id: &AutomationId) -> Result<Option<Automation>, DbError> {
        let row = sqlx::query_as!(
            AutomationRow,
            r#"SELECT id AS "id: AutomationId", name, enabled AS "enabled: bool",
                   auto_run AS "auto_run: bool", stop_on_failure AS "stop_on_failure: bool",
                   conditions_json AS "conditions: Json<ConditionGroups>",
                   steps_json AS "steps: Json<Vec<Action>>",
                   created_at AS "created_at: DateTime<Utc>",
                   updated_at AS "updated_at: DateTime<Utc>"
               FROM automations WHERE id = ?1"#,
            id
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(row.map(Automation::from))
    }

    async fn create(&self, automation: &Automation) -> Result<(), DbError> {
        let conditions = Json(&automation.conditions);
        let steps = Json(&automation.steps);
        sqlx::query!(
            "INSERT INTO automations (id, name, enabled, auto_run, stop_on_failure,
                 conditions_json, steps_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            automation.id,
            automation.name,
            automation.enabled,
            automation.auto_run,
            automation.stop_on_failure,
            conditions,
            steps,
            automation.created_at,
            automation.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn update(&self, automation: &Automation) -> Result<bool, DbError> {
        let conditions = Json(&automation.conditions);
        let steps = Json(&automation.steps);
        let result = sqlx::query!(
            "UPDATE automations SET name = ?1, enabled = ?2, auto_run = ?3,
                 stop_on_failure = ?4, conditions_json = ?5, steps_json = ?6, updated_at = ?7
             WHERE id = ?8",
            automation.name,
            automation.enabled,
            automation.auto_run,
            automation.stop_on_failure,
            conditions,
            steps,
            automation.updated_at,
            automation.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &AutomationId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM automations WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
