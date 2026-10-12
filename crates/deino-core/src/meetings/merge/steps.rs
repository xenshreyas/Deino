//! The SQL steps that fold one absorbed meeting into the merged one, each
//! run inside the merge's transaction. Order matters: speakers move before
//! the transcript (a segment's speaker must belong to its meeting, checked
//! by a trigger), and the absorbed row is deleted last, so its foreign keys
//! cascade only to what was not moved.

use deino_db::DbError;
use deino_domain::MeetingId;
use sqlx::SqliteConnection;

use super::{MergePart, MergePlan};

/// Records the part and re-homes the parts merged into it earlier.
pub(super) async fn record_part(
    connection: &mut SqliteConnection,
    plan: &MergePlan,
    part: &MergePart,
) -> Result<(), DbError> {
    let audio = &part.audio;
    sqlx::query!(
        "INSERT INTO meeting_merge_parts (meeting_id, position, source_meeting_id, title, held_at,
             offset_ms, duration_ms, audio_path, audio_mic_path, audio_system_path, video_path, merged_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        plan.target, part.position, part.source, part.title, part.held_at, part.offset,
        part.length, audio.audio_path, audio.audio_mic_path, audio.audio_system_path,
        audio.video_path, plan.merged_at,
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "UPDATE meeting_merge_parts
         SET meeting_id = ?1, position = position + ?2, offset_ms = offset_ms + ?3
         WHERE meeting_id = ?4",
        plan.target,
        part.position,
        part.offset,
        part.source,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Moves the part's speakers. One that is the user, or the same person as
/// a speaker the target already has, stays behind and its lines join that
/// speaker; a label the target already uses gets the part number.
pub(super) async fn move_speakers(
    connection: &mut SqliteConnection,
    target: &MeetingId,
    part: &MergePart,
) -> Result<(), DbError> {
    let suffix = format!(" ({})", part.position + 1);
    sqlx::query!(
        "UPDATE speakers AS s
         SET meeting_id = ?1,
             label = CASE WHEN EXISTS (
                 SELECT 1 FROM speakers AS t WHERE t.meeting_id = ?1 AND t.label = s.label
             ) THEN s.label || ?3 ELSE s.label END
         WHERE s.meeting_id = ?2 AND NOT EXISTS (
             SELECT 1 FROM speakers AS t
             WHERE t.meeting_id = ?1 AND ((t.is_me = 1 AND s.is_me = 1) OR t.person_id = s.person_id)
         )",
        target, part.source, suffix,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Moves the transcript, shifting every time (and each word's) by the
/// part's offset, onto the target's speakers.
pub(super) async fn move_transcript(
    connection: &mut SqliteConnection,
    target: &MeetingId,
    part: &MergePart,
) -> Result<(), DbError> {
    sqlx::query_file!(
        "src/meetings/merge/queries/move_transcript.sql",
        target,
        part.source,
        part.offset
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Moves attendance with the stronger role (organizer, then invitee, then
/// speaker only), as the person merge does (ADR 0036), plus tags, folders,
/// action items and voiceprint sources.
pub(super) async fn move_links(
    connection: &mut SqliteConnection,
    target: &MeetingId,
    source: &MeetingId,
) -> Result<(), DbError> {
    sqlx::query_file!(
        "src/meetings/merge/queries/move_attendance.sql",
        target,
        source
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "INSERT OR IGNORE INTO meeting_tags (meeting_id, tag_id, applied_by, applied_at)
         SELECT ?1, tag_id, applied_by, applied_at FROM meeting_tags WHERE meeting_id = ?2",
        target,
        source,
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "INSERT OR IGNORE INTO meeting_folders (meeting_id, folder_id, added_at)
         SELECT ?1, folder_id, added_at FROM meeting_folders WHERE meeting_id = ?2",
        target,
        source,
    )
    .execute(&mut *connection)
    .await?;
    move_owned_rows(connection, target, source).await
}

/// Re-points rows that reference the meeting without being its children.
async fn move_owned_rows(
    connection: &mut SqliteConnection,
    target: &MeetingId,
    source: &MeetingId,
) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE action_items SET meeting_id = ?1 WHERE meeting_id = ?2",
        target,
        source
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "UPDATE voiceprints SET source_meeting_id = ?1 WHERE source_meeting_id = ?2",
        target,
        source,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Deletes the absorbed meeting; what is left of it (its AI notes, jobs,
/// leftover speakers, legacy rows) cascades.
pub(super) async fn delete_source(
    connection: &mut SqliteConnection,
    source: &MeetingId,
) -> Result<(), DbError> {
    sqlx::query!("DELETE FROM meetings WHERE id = ?1", source)
        .execute(&mut *connection)
        .await?;
    Ok(())
}

/// Stamps the merged meeting with the whole timeline's length.
pub(super) async fn finish(
    connection: &mut SqliteConnection,
    plan: &MergePlan,
) -> Result<(), DbError> {
    let seconds = plan.length.get().div_ceil(1000);
    sqlx::query!(
        "UPDATE meetings SET duration_s = ?1, updated_at = ?2 WHERE id = ?3",
        seconds,
        plan.merged_at,
        plan.target,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}
