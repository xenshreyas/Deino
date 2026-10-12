//! The series choice port and its SQLite implementation
//! (`calendar_series_choices`). One event's own choice is
//! [`CalendarEventRepo::set_record_override`](crate::calendar::CalendarEventRepo::set_record_override).

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::SeriesId;

use crate::calendar::SqliteCalendarRepo;

/// Where the user's "all occurrences" choices are stored.
#[async_trait]
pub trait SeriesChoiceRepo: Send + Sync {
    /// Every series the user chose for: `true` always records, `false` never.
    async fn series_choices(&self) -> Result<BTreeMap<SeriesId, bool>, DbError>;

    /// Sets (`Some`) or clears (`None`) the choice for `series`; setting it
    /// also clears every occurrence's own choice, so the series' wins.
    async fn set_series_choice(
        &self,
        series: &SeriesId,
        record: Option<bool>,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;
}

#[async_trait]
impl SeriesChoiceRepo for SqliteCalendarRepo {
    async fn series_choices(&self) -> Result<BTreeMap<SeriesId, bool>, DbError> {
        let rows = sqlx::query!(
            r#"SELECT series_id AS "series_id: SeriesId", record AS "record: bool"
               FROM calendar_series_choices"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(|row| (row.series_id, row.record)).collect())
    }

    async fn set_series_choice(
        &self,
        series: &SeriesId,
        record: Option<bool>,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let mut tx = self.database.writer().begin().await?;
        let Some(record) = record else {
            sqlx::query!("DELETE FROM calendar_series_choices WHERE series_id = ?1", series)
                .execute(&mut *tx)
                .await?;
            return Ok(tx.commit().await?);
        };
        sqlx::query!(
            "INSERT INTO calendar_series_choices (series_id, record, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (series_id) DO UPDATE SET record = excluded.record, updated_at = excluded.updated_at",
            series,
            record,
            at,
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "UPDATE calendar_events SET record_override = NULL, updated_at = ?2
             WHERE series_id = ?1 AND record_override IS NOT NULL",
            series,
            at,
        )
        .execute(&mut *tx)
        .await?;
        Ok(tx.commit().await?)
    }
}
