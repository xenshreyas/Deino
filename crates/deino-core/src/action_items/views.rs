//! Saved views of the Action Items page: named filters shown as tabs
//! (Circleback §8 "Views"). [`SavedViewRepo`] stores them in
//! `action_item_views` (ADR 0028: a table rather than the settings
//! document, so each view is a row the UI refreshes through `DbChanged`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::ActionItemViewId;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use super::{ActionItemFilter, SqliteActionItemRepo};

/// A named filter, shown as a tab.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedView {
    /// The view's ID.
    pub id: ActionItemViewId,
    /// Its tab label; unique, ignoring case.
    pub name: String,
    /// What it shows.
    pub filter: ActionItemFilter,
    /// Its place among the tabs, from 0.
    pub position: u32,
    /// When it was saved first.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
}

/// What the user saves: a name and the filter on screen.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedViewInput {
    /// The tab label.
    pub name: String,
    /// The filter.
    pub filter: ActionItemFilter,
}

/// Where saved views are stored.
#[async_trait]
pub trait SavedViewRepo: Send + Sync {
    /// Every view, in tab order.
    async fn list_views(&self) -> Result<Vec<SavedView>, DbError>;

    /// Adds `view` after the last tab; its `position` is ignored.
    async fn create_view(&self, view: &SavedView) -> Result<(), DbError>;

    /// Saves the view's name, filter and `updated_at`.
    async fn update_view(&self, view: &SavedView) -> Result<bool, DbError>;

    /// Deletes the view.
    async fn delete_view(&self, id: &ActionItemViewId) -> Result<bool, DbError>;
}

/// One `action_item_views` row.
struct ViewRow {
    id: ActionItemViewId,
    name: String,
    filter: Json<ActionItemFilter>,
    position: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ViewRow {
    fn into_view(self) -> SavedView {
        SavedView {
            id: self.id,
            name: self.name,
            filter: self.filter.0,
            position: u32::try_from(self.position).unwrap_or(u32::MAX),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[async_trait]
impl SavedViewRepo for SqliteActionItemRepo {
    async fn list_views(&self) -> Result<Vec<SavedView>, DbError> {
        let rows = sqlx::query_as!(
            ViewRow,
            r#"SELECT
                id AS "id: ActionItemViewId",
                name,
                filter_json AS "filter: Json<ActionItemFilter>",
                position,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM action_item_views
            ORDER BY position, id"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(ViewRow::into_view).collect())
    }

    async fn create_view(&self, view: &SavedView) -> Result<(), DbError> {
        let filter = Json(&view.filter);
        sqlx::query!(
            "INSERT INTO action_item_views (id, name, filter_json, position, created_at, updated_at)
             VALUES (?1, ?2, ?3,
                 (SELECT coalesce(max(position) + 1, 0) FROM action_item_views), ?4, ?5)",
            view.id,
            view.name,
            filter,
            view.created_at,
            view.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn update_view(&self, view: &SavedView) -> Result<bool, DbError> {
        let filter = Json(&view.filter);
        let result = sqlx::query!(
            "UPDATE action_item_views SET name = ?1, filter_json = ?2, updated_at = ?3 WHERE id = ?4",
            view.name,
            filter,
            view.updated_at,
            view.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_view(&self, id: &ActionItemViewId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM action_item_views WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
