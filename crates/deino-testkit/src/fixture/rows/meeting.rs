//! The `meetings` row of a fixture, linked to its calendar event.

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingSource, MeetingStatus, Seconds};
use sqlx::SqliteConnection;

use super::super::ids::{event_key, fixture_uuid};
use super::super::model::FixtureMeeting;
use super::super::seeding::SeedTarget;
use super::super::vocab::Recurrence;

/// The meeting row: `ready`, recorded live, with its timing and calendar
/// link. `created_at` is when recording stopped, which is when the legacy
/// path saves a meeting. Re-seeding restores every column, including a
/// title the user changed, and takes the meeting out of the trash.
pub struct MeetingRows;

#[async_trait]
impl SeedTarget for MeetingRows {
    async fn seed(
        &self,
        connection: &mut SqliteConnection,
        meeting: &FixtureMeeting,
    ) -> Result<(), DbError> {
        let (started_at, ended_at) = (meeting.started_at, meeting.ended_at());
        let (status, source) = (MeetingStatus::Ready, MeetingSource::Live);
        let duration = Seconds(meeting.duration.0.saturating_add(500) / 1000);
        let event = fixture_uuid(&event_key(meeting.id.as_str()));
        let series = series_id(meeting);
        sqlx::query_file!(
            "src/fixture/rows/queries/meeting.sql",
            meeting.id,
            meeting.title,
            ended_at,
            status,
            source,
            started_at,
            duration,
            event,
            series,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
}

/// The recurring series a fixture belongs to: its iCalendar UID, shared by
/// every occurrence, or `None` for a one-off event.
pub(super) fn series_id(meeting: &FixtureMeeting) -> Option<&str> {
    let event = &meeting.calendar_event;
    (event.recurrence != Recurrence::None).then_some(event.ical_uid.as_str())
}
