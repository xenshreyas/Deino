//! Saved views of the Meetings page: named filters shown as tabs
//! (Circleback §14 "Views"). [`MeetingViewRepo`] stores them in
//! `meeting_views`, a table rather than the settings document, so each view
//! is a row the UI refreshes through `DbChanged` (as AI-05's views).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::MeetingViewId;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use super::{MeetingFilter, SqliteMeetingRepo};

/// A named filter, shown as a tab.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingView {
    /// The view's ID.
    pub id: MeetingViewId,
    /// Its tab label; unique, ignoring case.
    pub name: String,
    /// What it shows.
    pub filter: MeetingFilter,
    /// Its place among the tabs, from 0.
    pub position: u32,
    /// When it was first saved.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
}

/// What the user saves: a name and the filter on screen.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingViewInput {
    /// The tab label.
    pub name: String,
    /// The filter.
    pub filter: MeetingFilter,
}

/// Where saved views are stored.
#[async_trait]
pub trait MeetingViewRepo: Send + Sync {
    /// Every view, in tab order.
    async fn list_views(&self) -> Result<Vec<MeetingView>, DbError>;

    /// Adds `view` after the last tab; its `position` is ignored.
    async fn create_view(&self, view: &MeetingView) -> Result<(), DbError>;

    /// Saves the view's name, filter and `updated_at`.
    async fn update_view(&self, view: &MeetingView) -> Result<bool, DbError>;

    /// Deletes the view.
    async fn delete_view(&self, id: &MeetingViewId) -> Result<bool, DbError>;
}

/// One `meeting_views` row.
struct ViewRow {
    id: MeetingViewId,
    name: String,
    filter: Json<MeetingFilter>,
    position: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ViewRow {
    fn into_view(self) -> MeetingView {
        MeetingView {
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
impl MeetingViewRepo for SqliteMeetingRepo {
    async fn list_views(&self) -> Result<Vec<MeetingView>, DbError> {
        let rows = sqlx::query_as!(
            ViewRow,
            r#"SELECT
                id AS "id: MeetingViewId",
                name,
                filter_json AS "filter: Json<MeetingFilter>",
                position,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM meeting_views
            ORDER BY position, id"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(ViewRow::into_view).collect())
    }

    async fn create_view(&self, view: &MeetingView) -> Result<(), DbError> {
        let filter = Json(&view.filter);
        sqlx::query!(
            "INSERT INTO meeting_views (id, name, filter_json, position, created_at, updated_at)
             VALUES (?1, ?2, ?3,
                 (SELECT coalesce(max(position) + 1, 0) FROM meeting_views), ?4, ?5)",
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

    async fn update_view(&self, view: &MeetingView) -> Result<bool, DbError> {
        let filter = Json(&view.filter);
        let result = sqlx::query!(
            "UPDATE meeting_views SET name = ?1, filter_json = ?2, updated_at = ?3 WHERE id = ?4",
            view.name,
            filter,
            view.updated_at,
            view.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_view(&self, id: &MeetingViewId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM meeting_views WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
