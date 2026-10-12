//! The statements that write a [`Seating`] (CAP-05), inside the caller's
//! transaction ([`SqliteNamingRepo`](super::SqliteNamingRepo)).
//!
//! Each statement re-checks in SQL that the speaker it touches is still
//! unnamed, so a rename the user made after the stage read the meeting is
//! never overwritten.

use deino_db::DbError;
use deino_domain::{DisplayName, MeetingId, SpeakerId};
use sqlx::SqliteConnection;

use super::naming_sql::insert_speaker;
use super::{Seating, SegmentMove};

/// Writes `seating` for `meeting`: new speakers, labels, moved segments,
/// then the speakers the moves emptied.
pub(super) async fn apply(
    connection: &mut SqliteConnection,
    meeting: &MeetingId,
    seating: &Seating,
) -> Result<(), DbError> {
    for speaker in &seating.created {
        insert_speaker(&mut *connection, speaker).await?;
    }
    for (speaker, label) in &seating.labelled {
        relabel(&mut *connection, meeting, speaker, label).await?;
    }
    for change in &seating.moved {
        move_segment(&mut *connection, meeting, change).await?;
    }
    for speaker in &seating.emptied {
        drop_if_empty(&mut *connection, meeting, speaker).await?;
    }
    Ok(())
}

/// Shows an unnamed speaker under `label`, unless another speaker of the
/// meeting already has that label.
async fn relabel(
    connection: &mut SqliteConnection,
    meeting: &MeetingId,
    speaker: &SpeakerId,
    label: &DisplayName,
) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE speakers SET label = ?1
         WHERE id = ?2 AND meeting_id = ?3 AND person_id IS NULL
           AND NOT EXISTS (SELECT 1 FROM speakers WHERE meeting_id = ?3 AND label = ?1)",
        label,
        speaker,
        meeting,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Points a segment at its name's speaker, if it sits on no speaker or on
/// one nobody named that is not the user's.
async fn move_segment(
    connection: &mut SqliteConnection,
    meeting: &MeetingId,
    change: &SegmentMove,
) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE transcript_segments SET speaker_id = ?1
         WHERE id = ?2 AND meeting_id = ?3
           AND (speaker_id IS NULL OR speaker_id IN (
                SELECT id FROM speakers
                WHERE meeting_id = ?3 AND person_id IS NULL AND is_me = 0))",
        change.to,
        change.segment,
        meeting,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Removes an unnamed speaker that no segment points at any more.
async fn drop_if_empty(
    connection: &mut SqliteConnection,
    meeting: &MeetingId,
    speaker: &SpeakerId,
) -> Result<(), DbError> {
    sqlx::query!(
        "DELETE FROM speakers
         WHERE id = ?1 AND meeting_id = ?2 AND person_id IS NULL AND is_me = 0
           AND NOT EXISTS (SELECT 1 FROM transcript_segments WHERE speaker_id = ?1)",
        speaker,
        meeting,
    )
    .execute(connection)
    .await?;
    Ok(())
}
