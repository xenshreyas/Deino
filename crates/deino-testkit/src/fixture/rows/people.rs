//! A fixture's companies, people and attendance.

use async_trait::async_trait;
use deino_db::DbError;
use sqlx::SqliteConnection;

use super::super::ids::{company_key, fixture_uuid, person_key};
use super::super::model::{Attendee, Company, FixtureMeeting};
use super::super::seeding::SeedTarget;

/// Companies by domain and people by email, upserted the way ORG-04 will
/// upsert calendar attendees, then the meeting's `meeting_attendees` rows,
/// replaced on every seed. People and companies are shared across meetings,
/// so they are never deleted; the user is not marked `is_me`, which belongs
/// to the real user's own person.
pub struct PeopleRows;

#[async_trait]
impl SeedTarget for PeopleRows {
    async fn seed(
        &self,
        connection: &mut SqliteConnection,
        meeting: &FixtureMeeting,
    ) -> Result<(), DbError> {
        for company in &meeting.companies {
            upsert_company(connection, company, meeting).await?;
        }
        sqlx::query!(
            "DELETE FROM meeting_attendees WHERE meeting_id = ?1",
            meeting.id
        )
        .execute(&mut *connection)
        .await?;
        for attendee in &meeting.attendees {
            upsert_person(connection, attendee, meeting).await?;
            sqlx::query!(
                "INSERT INTO meeting_attendees (meeting_id, person_id, role, rsvp)
                 VALUES (?1, (SELECT id FROM people WHERE email = ?2), ?3, ?4)",
                meeting.id,
                attendee.email,
                attendee.role,
                attendee.rsvp,
            )
            .execute(&mut *connection)
            .await?;
        }
        Ok(())
    }
}

async fn upsert_company(
    connection: &mut SqliteConnection,
    company: &Company,
    meeting: &FixtureMeeting,
) -> Result<(), DbError> {
    let id = fixture_uuid(&company_key(&company.domain));
    sqlx::query!(
        "INSERT INTO companies (id, name, domain, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT (domain) DO UPDATE SET name = excluded.name",
        id,
        company.name,
        company.domain,
        meeting.started_at,
    )
    .execute(connection)
    .await?;
    Ok(())
}

async fn upsert_person(
    connection: &mut SqliteConnection,
    attendee: &Attendee,
    meeting: &FixtureMeeting,
) -> Result<(), DbError> {
    let id = fixture_uuid(&person_key(attendee.email.as_str()));
    let domain = attendee.email.domain();
    sqlx::query!(
        "INSERT INTO people (id, name, email, company_id, title, created_at, updated_at)
         VALUES (?1, ?2, ?3, (SELECT id FROM companies WHERE domain = ?4), ?5, ?6, ?6)
         ON CONFLICT (email) DO UPDATE SET
             name = excluded.name, company_id = excluded.company_id, title = excluded.title",
        id,
        attendee.name,
        attendee.email,
        domain,
        attendee.title,
        meeting.started_at,
    )
    .execute(connection)
    .await?;
    Ok(())
}
