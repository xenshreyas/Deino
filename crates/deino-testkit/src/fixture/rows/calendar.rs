//! The calendar event a fixture was recorded from, in the fixture account's
//! "Work" calendar.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{CalendarProvider, ConferencePlatform, Rsvp};
use serde_json::{Value, json};
use sqlx::SqliteConnection;
use sqlx::types::Json;

use super::super::ids::{
    CALENDAR_ACCOUNT_NAME, CALENDAR_COLOR, CALENDAR_KEY, CALENDAR_NAME, event_key, fixture_uuid,
};
use super::super::model::{FixtureMeeting, USER_EMAIL};
use super::super::seeding::SeedTarget;
use super::super::vocab::{AttendeeRole, Email, Platform};
use super::meeting::series_id;

/// The fixture calendar account (`EventKit`, "Deino fixtures"), its "Work"
/// calendar and one event per fixture meeting, upserted by its provider ID
/// like a real sync.
pub struct CalendarRows;

#[async_trait]
impl SeedTarget for CalendarRows {
    async fn seed(
        &self,
        connection: &mut SqliteConnection,
        meeting: &FixtureMeeting,
    ) -> Result<(), DbError> {
        let provider = CalendarProvider::Apple;
        let account = fixture_uuid("calendar-account");
        sqlx::query!(
            "INSERT INTO calendar_accounts (id, provider, name, created_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT DO NOTHING",
            account,
            provider,
            CALENDAR_ACCOUNT_NAME,
            meeting.started_at,
        )
        .execute(&mut *connection)
        .await?;
        let calendar = fixture_uuid(CALENDAR_KEY);
        sqlx::query!(
            "INSERT INTO calendars (id, account_id, provider_calendar_id, title, color, created_at, updated_at)
             VALUES (?1, ?2, 'work', ?3, ?4, ?5, ?5)
             ON CONFLICT DO NOTHING",
            calendar,
            account,
            CALENDAR_NAME,
            CALENDAR_COLOR,
            meeting.started_at,
        )
        .execute(&mut *connection)
        .await?;
        EventRow::of(meeting).upsert(connection).await
    }
}

/// The columns of a fixture's `calendar_events` row.
struct EventRow<'a> {
    id: String,
    meeting: &'a FixtureMeeting,
    platform: Option<ConferencePlatform>,
    attendees: Json<Vec<Value>>,
    my_rsvp: Option<Rsvp>,
    is_external: bool,
    synced_at: DateTime<Utc>,
}

impl<'a> EventRow<'a> {
    fn of(meeting: &'a FixtureMeeting) -> Self {
        let user = meeting
            .attendees
            .iter()
            .find(|a| a.email.as_str() == USER_EMAIL);
        let user_domain = USER_EMAIL.split_once('@').map_or("", |(_, domain)| domain);
        Self {
            id: fixture_uuid(&event_key(meeting.id.as_str())),
            meeting,
            platform: conference_platform(meeting.calendar_event.platform),
            attendees: Json(meeting.attendees.iter().map(attendee_json).collect()),
            my_rsvp: user.map(|attendee| attendee.rsvp),
            is_external: meeting
                .attendees
                .iter()
                .any(|a| a.email.domain() != user_domain),
            synced_at: meeting.started_at,
        }
    }

    async fn upsert(&self, connection: &mut SqliteConnection) -> Result<(), DbError> {
        let event = &self.meeting.calendar_event;
        let organizer: &Email = &event.organizer_email;
        let series = series_id(self.meeting);
        let calendar = fixture_uuid(CALENDAR_KEY);
        sqlx::query_file!(
            "src/fixture/rows/queries/calendar_event.sql",
            self.id,
            CALENDAR_ACCOUNT_NAME,
            event.id,
            event.ical_uid,
            series,
            event.title,
            event.description,
            event.start_at,
            event.end_at,
            event.conference_url,
            self.platform,
            organizer,
            self.attendees,
            self.my_rsvp,
            self.is_external,
            self.synced_at,
            calendar,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
}

/// The video-call service of a fixture venue; `None` in person.
fn conference_platform(platform: Platform) -> Option<ConferencePlatform> {
    match platform {
        Platform::GoogleMeet => Some(ConferencePlatform::GoogleMeet),
        Platform::Zoom => Some(ConferencePlatform::Zoom),
        Platform::Teams => Some(ConferencePlatform::Teams),
        Platform::InPerson => None,
    }
}

/// One invitee as `calendar_events.attendees_json` stores it (the shape of
/// `deino_core::calendar::EventAttendee`).
fn attendee_json(attendee: &super::super::model::Attendee) -> Value {
    json!({
        "email": attendee.email,
        "name": attendee.name,
        "rsvp": attendee.rsvp,
        "isOrganizer": attendee.role == AttendeeRole::Organizer,
    })
}
