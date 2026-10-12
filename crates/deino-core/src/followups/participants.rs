//! [`ParticipantReader`]: who took part in a meeting, from ORG-04's people
//! and companies: every attendee and every speaker named after a person,
//! once each.

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{Email, MeetingId, PersonId};

use super::{Participant, SqliteFollowUpRepo};

/// Reads a meeting's participants.
#[async_trait]
pub trait ParticipantReader: Send + Sync {
    /// Everyone who took part in `meeting`, the user first, then by name.
    async fn participants(&self, meeting: &MeetingId) -> Result<Vec<Participant>, DbError>;
}

#[async_trait]
impl ParticipantReader for SqliteFollowUpRepo {
    async fn participants(&self, meeting: &MeetingId) -> Result<Vec<Participant>, DbError> {
        let rows = sqlx::query!(
            r#"SELECT
                p.id AS "id!: PersonId",
                p.name AS "name!",
                p.email AS "email: Email",
                p.is_me AS "is_me!: bool",
                c.name AS "company?: String"
            FROM people AS p
            LEFT JOIN companies AS c ON c.id = p.company_id
            WHERE p.id IN (
                SELECT person_id FROM meeting_attendees WHERE meeting_id = ?1
                UNION
                SELECT person_id FROM speakers
                WHERE meeting_id = ?1 AND person_id IS NOT NULL
            )
            ORDER BY p.is_me DESC, p.name COLLATE NOCASE"#,
            meeting,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| Participant {
                id: row.id,
                name: row.name,
                email: row.email,
                company: row.company,
                is_me: row.is_me,
            })
            .collect())
    }
}
