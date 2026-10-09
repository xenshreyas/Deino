//! The attendee port: who took part in which meeting.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{AttendeeRole, MeetingId, PersonId, Rsvp};

use super::{Attendee, SqlitePeopleRepo};
use crate::meetings::Meeting;

/// Links people to the meetings they attended.
#[async_trait]
pub trait AttendeeRepo: Send + Sync {
    /// Adds the person to the meeting, or updates their role and RSVP if
    /// they are already on it.
    async fn upsert_attendee(&self, attendee: &Attendee) -> Result<(), DbError>;

    /// Takes the person off the meeting.
    async fn remove_attendee(
        &self,
        meeting: &MeetingId,
        person: &PersonId,
    ) -> Result<bool, DbError>;

    /// The meeting's attendees, in the order they were first added.
    async fn attendees(&self, meeting: &MeetingId) -> Result<Vec<Attendee>, DbError>;

    /// The meetings outside the trash the person attended, newest first.
    async fn meetings_of(&self, person: &PersonId) -> Result<Vec<Meeting>, DbError>;
}

#[async_trait]
impl AttendeeRepo for SqlitePeopleRepo {
    async fn upsert_attendee(&self, attendee: &Attendee) -> Result<(), DbError> {
        // ON CONFLICT DO UPDATE, never REPLACE: the update hook misses
        // REPLACE deletions, and DbChanged must see every write.
        sqlx::query!(
            "INSERT INTO meeting_attendees (meeting_id, person_id, role, rsvp) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (meeting_id, person_id) DO UPDATE SET role = excluded.role, rsvp = excluded.rsvp",
            attendee.meeting_id,
            attendee.person_id,
            attendee.role,
            attendee.rsvp,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn remove_attendee(
        &self,
        meeting: &MeetingId,
        person: &PersonId,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "DELETE FROM meeting_attendees WHERE meeting_id = ?1 AND person_id = ?2",
            meeting,
            person,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn attendees(&self, meeting: &MeetingId) -> Result<Vec<Attendee>, DbError> {
        let attendees = sqlx::query_file_as!(Attendee, "src/people/queries/attendees.sql", meeting)
            .fetch_all(self.database.reader())
            .await?;
        Ok(attendees)
    }

    async fn meetings_of(&self, person: &PersonId) -> Result<Vec<Meeting>, DbError> {
        let meetings = sqlx::query_file_as!(Meeting, "src/people/queries/meetings_of.sql", person)
            .fetch_all(self.database.reader())
            .await?;
        Ok(meetings)
    }
}
