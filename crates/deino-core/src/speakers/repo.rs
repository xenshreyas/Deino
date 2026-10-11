//! The speaker naming ports and their SQLite implementation.
//!
//! Reads ([`ParticipantReader`]), the user's naming writes ([`NamingRepo`])
//! and the `identify_speakers` stage's writes ([`IdentificationWriter`]) are
//! separate ports, so each caller depends only on what it uses. Every write
//! that touches several rows is one transaction ([`naming_sql`](super::naming_sql)).

use async_trait::async_trait;
use deino_db::{Database, DbError};
use deino_domain::{MeetingId, Millis, PaletteColor, PersonId, SegmentId, SpeakerId};

use super::naming_sql;
use super::{
    Identification, MergeSpeakers, NameSpeaker, ParticipantRow, PersonRef, SegmentTarget,
    Suggestion,
};
use crate::segments::Speaker;

/// Reads a meeting's participants.
#[async_trait]
pub trait ParticipantReader: Send + Sync {
    /// The meeting's speakers with who they are, their speech and any
    /// suggestion; most speech first, then by label.
    async fn participants(&self, meeting: &MeetingId) -> Result<Vec<ParticipantRow>, DbError>;
}

/// The user's naming changes.
#[async_trait]
pub trait NamingRepo: Send + Sync {
    /// Makes the speaker the person, drops its suggestion, and moves the
    /// meeting's action items from the previous person to the new one when
    /// no other speaker still is the previous person. Returns `false`,
    /// changing nothing, when the meeting has no such speaker.
    async fn name_speaker(&self, change: &NameSpeaker) -> Result<bool, DbError>;

    /// Moves one segment of `meeting` to `target`, creating it if new.
    /// Returns `false`, changing nothing, when the meeting has no such
    /// segment.
    async fn name_segment(
        &self,
        meeting: &MeetingId,
        segment: &SegmentId,
        target: &SegmentTarget,
    ) -> Result<bool, DbError>;

    /// Moves every segment of `from` to `into`, gives `into` the person of
    /// `from` when it had none, moves action items like
    /// [`NamingRepo::name_speaker`], and deletes `from`.
    async fn merge(&self, change: &MergeSpeakers) -> Result<(), DbError>;

    /// Drops the speaker's suggestion; `false` when it had none.
    async fn dismiss_suggestion(&self, speaker: &SpeakerId) -> Result<bool, DbError>;
}

/// The `identify_speakers` stage's writes.
#[async_trait]
pub trait IdentificationWriter: Send + Sync {
    /// Names the speakers and replaces the meeting's suggestions, in one
    /// transaction.
    async fn apply(&self, meeting: &MeetingId, decided: &Identification) -> Result<(), DbError>;
}

/// [`ParticipantReader`], [`NamingRepo`] and [`IdentificationWriter`] over
/// Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteNamingRepo {
    database: Database,
}

impl SqliteNamingRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ParticipantReader for SqliteNamingRepo {
    async fn participants(&self, meeting: &MeetingId) -> Result<Vec<ParticipantRow>, DbError> {
        let rows = sqlx::query_file!("src/speakers/queries/participants.sql", meeting)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| ParticipantRow {
                person: row.person_id.zip(row.person_name).map(person_ref),
                suggestion: suggestion(row.suggested_id, row.suggested_name, row.suggested_score),
                speech: row.speech,
                speaker: Speaker {
                    id: row.id,
                    meeting_id: row.meeting_id,
                    label: row.label,
                    person_id: row.person_id,
                    is_me: row.is_me,
                    color: row.color,
                },
            })
            .collect())
    }
}

#[async_trait]
impl NamingRepo for SqliteNamingRepo {
    async fn name_speaker(&self, change: &NameSpeaker) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        if !naming_sql::set_person(&mut transaction, change).await? {
            return Ok(false);
        }
        if let Some((from, to)) = handover(change.previous, Some(change.person)) {
            let moved = naming_sql::Reassign {
                meeting: &change.meeting_id,
                from: &from,
                to: &to,
                at: change.at,
            };
            naming_sql::reassign_items(&mut transaction, &moved).await?;
        }
        transaction.commit().await?;
        Ok(true)
    }

    async fn name_segment(
        &self,
        meeting: &MeetingId,
        segment: &SegmentId,
        target: &SegmentTarget,
    ) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let speaker = match target {
            SegmentTarget::Existing(id) => *id,
            SegmentTarget::New(speaker) => {
                naming_sql::insert_speaker(&mut transaction, speaker).await?;
                speaker.id
            }
        };
        if !naming_sql::move_segment(&mut transaction, meeting, segment, &speaker).await? {
            return Ok(false);
        }
        transaction.commit().await?;
        Ok(true)
    }

    async fn merge(&self, change: &MergeSpeakers) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        naming_sql::fold_speaker(&mut transaction, change).await?;
        let from = change.from.person_id;
        if let Some((from, to)) = handover(from, change.into.person_id.or(from)) {
            let moved = naming_sql::Reassign {
                meeting: &change.meeting_id,
                from: &from,
                to: &to,
                at: change.at,
            };
            naming_sql::reassign_items(&mut transaction, &moved).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn dismiss_suggestion(&self, speaker: &SpeakerId) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "DELETE FROM speaker_suggestions WHERE speaker_id = ?1",
            speaker
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait]
impl IdentificationWriter for SqliteNamingRepo {
    async fn apply(&self, meeting: &MeetingId, decided: &Identification) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for (speaker, person) in &decided.named {
            naming_sql::assign_if_unnamed(&mut transaction, speaker, person).await?;
        }
        naming_sql::replace_suggestions(&mut transaction, meeting, &decided.suggested).await?;
        transaction.commit().await?;
        Ok(())
    }
}

/// Whose action items move to whom when a voice changes person: only
/// between two different, known people.
fn handover(from: Option<PersonId>, to: Option<PersonId>) -> Option<(PersonId, PersonId)> {
    match (from, to) {
        (Some(from), Some(to)) if from != to => Some((from, to)),
        _ => None,
    }
}

fn person_ref((id, name): (PersonId, String)) -> PersonRef {
    PersonRef { id, name }
}

fn suggestion(
    id: Option<PersonId>,
    name: Option<String>,
    score: Option<f64>,
) -> Option<Suggestion> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "stored from an f32 cosine similarity, so it converts back exactly"
    )]
    let score = score? as f32;
    Some(Suggestion {
        person: person_ref((id?, name?)),
        score,
    })
}
