//! The calendar event port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{
    CalendarAccountId, CalendarEventId, CalendarId, ConferencePlatform, Rsvp, SeriesId,
};
use sqlx::SqliteConnection;
use sqlx::types::Json;

use super::{CalendarEvent, EventAttendee, SqliteCalendarRepo};

/// Where synced calendar events are stored.
#[async_trait]
pub trait CalendarEventRepo: Send + Sync {
    /// Stores `event`, or refreshes the stored copy with the same account
    /// and provider ID. A refresh keeps the stored ID, `created_at` and the
    /// user's `record_override`. Returns the stored event's ID.
    async fn upsert_event(&self, event: &CalendarEvent) -> Result<CalendarEventId, DbError>;

    /// The event with `id`.
    async fn event(&self, id: &CalendarEventId) -> Result<Option<CalendarEvent>, DbError>;

    /// Events starting at or after `from` and before `to`, by start time.
    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, DbError>;

    /// Sets whether the event is always, never or rule-based recorded.
    async fn set_record_override(
        &self,
        id: &CalendarEventId,
        record: Option<bool>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Deletes the event; a meeting recorded for it keeps its series and
    /// loses the link.
    async fn delete_event(&self, id: &CalendarEventId) -> Result<bool, DbError>;
}

#[async_trait]
impl CalendarEventRepo for SqliteCalendarRepo {
    async fn upsert_event(&self, event: &CalendarEvent) -> Result<CalendarEventId, DbError> {
        let mut connection = self.database.writer().acquire().await?;
        if let Some(id) = upsert_on(&mut connection, event).await? {
            return Ok(id);
        }
        // Unchanged: the upsert wrote nothing and returned nothing.
        let id = sqlx::query_scalar!(
            r#"SELECT id AS "id: CalendarEventId" FROM calendar_events
            WHERE account_id = ?1 AND provider_event_id = ?2"#,
            event.account_id,
            event.provider_event_id,
        )
        .fetch_one(&mut *connection)
        .await?;
        Ok(id)
    }

    async fn event(&self, id: &CalendarEventId) -> Result<Option<CalendarEvent>, DbError> {
        let row = sqlx::query_file_as!(EventRow, "src/calendar/queries/event.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(row.map(CalendarEvent::from))
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, DbError> {
        let rows = sqlx::query_file_as!(
            EventRow,
            "src/calendar/queries/events_between.sql",
            from,
            to
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(CalendarEvent::from).collect())
    }

    async fn set_record_override(
        &self,
        id: &CalendarEventId,
        record: Option<bool>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE calendar_events SET record_override = ?1, updated_at = ?2 WHERE id = ?3",
            record,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_event(&self, id: &CalendarEventId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM calendar_events WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// Stores `event`, or refreshes the stored copy with its account and
/// provider ID when the provider changed something. Returns the stored ID
/// when a row was written, `None` when the stored copy was already current.
/// Shared by [`CalendarEventRepo::upsert_event`] and the sync transaction.
pub(super) async fn upsert_on(
    connection: &mut SqliteConnection,
    event: &CalendarEvent,
) -> Result<Option<CalendarEventId>, DbError> {
    let attendees = Json(&event.attendees);
    let id = sqlx::query_file_scalar!(
        "src/calendar/queries/upsert_event.sql",
        event.id,
        event.account_id,
        event.calendar_id,
        event.provider_event_id,
        event.ical_uid,
        event.series_id,
        event.title,
        event.description,
        event.start_at,
        event.end_at,
        event.is_all_day,
        event.conference_url,
        event.platform,
        event.organizer_email,
        attendees,
        event.my_rsvp,
        event.is_external,
        event.record_override,
        event.created_at,
        event.updated_at,
    )
    .fetch_optional(connection)
    .await?;
    Ok(id)
}

/// One `calendar_events` row as the queries read it.
struct EventRow {
    id: CalendarEventId,
    account_id: CalendarAccountId,
    calendar_id: Option<CalendarId>,
    provider_event_id: String,
    ical_uid: Option<String>,
    series_id: Option<SeriesId>,
    title: String,
    description: String,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    is_all_day: bool,
    conference_url: Option<String>,
    platform: Option<ConferencePlatform>,
    organizer_email: Option<String>,
    attendees: Json<Vec<EventAttendee>>,
    my_rsvp: Option<Rsvp>,
    is_external: bool,
    record_override: Option<bool>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EventRow> for CalendarEvent {
    fn from(row: EventRow) -> Self {
        Self {
            id: row.id,
            account_id: row.account_id,
            calendar_id: row.calendar_id,
            provider_event_id: row.provider_event_id,
            ical_uid: row.ical_uid,
            series_id: row.series_id,
            title: row.title,
            description: row.description,
            start_at: row.start_at,
            end_at: row.end_at,
            is_all_day: row.is_all_day,
            conference_url: row.conference_url,
            platform: row.platform,
            organizer_email: row.organizer_email,
            attendees: row.attendees.0,
            my_rsvp: row.my_rsvp,
            is_external: row.is_external,
            record_override: row.record_override,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
