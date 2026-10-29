//! The statements of the naming transactions, each run inside the caller's
//! transaction ([`SqliteNamingRepo`](super::SqliteNamingRepo)).

use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{MeetingId, PersonId, SegmentId, SpeakerId};
use sqlx::SqliteConnection;

use super::{MergeSpeakers, NameSpeaker, SuggestedName};
use crate::segments::Speaker;

/// Action items of one meeting moving from one person to another.
pub(super) struct Reassign<'a> {
    pub(super) meeting: &'a MeetingId,
    pub(super) from: &'a PersonId,
    pub(super) to: &'a PersonId,
    pub(super) at: DateTime<Utc>,
}

/// Makes the speaker the person and drops its suggestion; `false` when the
/// meeting has no such speaker.
pub(super) async fn set_person(
    connection: &mut SqliteConnection,
    change: &NameSpeaker,
) -> Result<bool, DbError> {
    let updated = sqlx::query!(
        "UPDATE speakers SET person_id = ?1 WHERE id = ?2 AND meeting_id = ?3",
        change.person,
        change.speaker_id,
        change.meeting_id,
    )
    .execute(&mut *connection)
    .await?;
    if updated.rows_affected() == 0 {
        return Ok(false);
    }
    sqlx::query!(
        "DELETE FROM speaker_suggestions WHERE speaker_id = ?1",
        change.speaker_id
    )
    .execute(connection)
    .await?;
    Ok(true)
}

/// Moves the meeting's action items from one person to another, unless a
/// speaker of the meeting is still the first person (then the items may
/// well be theirs). Stamps `updated_at`, so AI-05's regeneration treats
/// them as touched.
pub(super) async fn reassign_items(
    connection: &mut SqliteConnection,
    change: &Reassign<'_>,
) -> Result<u64, DbError> {
    let moved = sqlx::query!(
        "UPDATE action_items SET assignee_person_id = ?1, updated_at = ?2
         WHERE meeting_id = ?3 AND assignee_person_id = ?4
           AND NOT EXISTS (SELECT 1 FROM speakers WHERE meeting_id = ?3 AND person_id = ?4)",
        change.to,
        change.at,
        change.meeting,
        change.from,
    )
    .execute(connection)
    .await?;
    Ok(moved.rows_affected())
}

/// Adds a speaker created by hand (a renamed segment's new owner).
pub(super) async fn insert_speaker(
    connection: &mut SqliteConnection,
    speaker: &Speaker,
) -> Result<(), DbError> {
    sqlx::query!(
        "INSERT INTO speakers (id, meeting_id, label, person_id, is_me, color)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        speaker.id,
        speaker.meeting_id,
        speaker.label,
        speaker.person_id,
        speaker.is_me,
        speaker.color,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Points one segment of the meeting at `speaker`; `false` when the meeting
/// has no such segment.
pub(super) async fn move_segment(
    connection: &mut SqliteConnection,
    meeting: &MeetingId,
    segment: &SegmentId,
    speaker: &SpeakerId,
) -> Result<bool, DbError> {
    let moved = sqlx::query!(
        "UPDATE transcript_segments SET speaker_id = ?1 WHERE id = ?2 AND meeting_id = ?3",
        speaker,
        segment,
        meeting,
    )
    .execute(connection)
    .await?;
    Ok(moved.rows_affected() > 0)
}

/// Moves every segment of `from` to `into`, hands `from`'s person to an
/// unnamed `into`, then deletes `from` (its suggestion goes with it).
pub(super) async fn fold_speaker(
    connection: &mut SqliteConnection,
    change: &MergeSpeakers,
) -> Result<(), DbError> {
    let (from, into) = (&change.from, &change.into);
    sqlx::query!(
        "UPDATE transcript_segments SET speaker_id = ?1 WHERE speaker_id = ?2",
        into.id,
        from.id,
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "UPDATE speakers SET person_id = ?1 WHERE id = ?2 AND person_id IS NULL",
        from.person_id,
        into.id,
    )
    .execute(&mut *connection)
    .await?;
    keep_voice(connection, from, into).await?;
    sqlx::query!("DELETE FROM speakers WHERE id = ?1", from.id)
        .execute(connection)
        .await?;
    Ok(())
}

/// Gives `into` the voice of `from` when it has none, or when `from`'s was
/// heard for longer: the merged speaker keeps the voice diarization found
/// for most of its speech, so naming it later can save a voice profile.
async fn keep_voice(
    connection: &mut SqliteConnection,
    from: &Speaker,
    into: &Speaker,
) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE speakers
         SET centroid = voice.centroid,
             embedding_model = voice.embedding_model,
             speech_ms = voice.speech_ms
         FROM (SELECT centroid, embedding_model, speech_ms FROM speakers WHERE id = ?1) AS voice
         WHERE speakers.id = ?2
           AND voice.centroid IS NOT NULL
           AND (speakers.centroid IS NULL OR voice.speech_ms > speakers.speech_ms)",
        from.id,
        into.id,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Names a speaker the user has not named in the meantime.
pub(super) async fn assign_if_unnamed(
    connection: &mut SqliteConnection,
    speaker: &SpeakerId,
    person: &PersonId,
) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE speakers SET person_id = ?1 WHERE id = ?2 AND person_id IS NULL",
        person,
        speaker,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Replaces the meeting's suggestions with `suggested`.
pub(super) async fn replace_suggestions(
    connection: &mut SqliteConnection,
    meeting: &MeetingId,
    suggested: &[SuggestedName],
) -> Result<(), DbError> {
    sqlx::query!(
        "DELETE FROM speaker_suggestions
         WHERE speaker_id IN (SELECT id FROM speakers WHERE meeting_id = ?1)",
        meeting
    )
    .execute(&mut *connection)
    .await?;
    for suggestion in suggested {
        let score = f64::from(suggestion.score);
        sqlx::query!(
            "INSERT INTO speaker_suggestions (speaker_id, person_id, score) VALUES (?1, ?2, ?3)",
            suggestion.speaker_id,
            suggestion.person_id,
            score,
        )
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}
