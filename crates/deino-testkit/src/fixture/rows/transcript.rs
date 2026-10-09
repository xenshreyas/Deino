//! A fixture's speakers and transcript segments.

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::PaletteColor;
use serde_json::{Value, json};
use sqlx::SqliteConnection;
use sqlx::types::Json;

use super::super::ids::{fixture_uuid, segment_key, speaker_key};
use super::super::model::{FixtureMeeting, Segment, Speaker, USER_EMAIL};
use super::super::seeding::SeedTarget;

/// The meeting's `speakers` (linked to the people they are, the user's
/// voice marked `is_me`) and its `transcript_segments` with their channel
/// and word timings, replaced on every seed. Legacy code reads the same rows
/// through the `transcripts` compatibility view.
pub struct TranscriptRows;

#[async_trait]
impl SeedTarget for TranscriptRows {
    async fn seed(
        &self,
        connection: &mut SqliteConnection,
        meeting: &FixtureMeeting,
    ) -> Result<(), DbError> {
        let id = &meeting.id;
        sqlx::query!("DELETE FROM transcript_segments WHERE meeting_id = ?1", id)
            .execute(&mut *connection)
            .await?;
        sqlx::query!("DELETE FROM speakers WHERE meeting_id = ?1", id)
            .execute(&mut *connection)
            .await?;
        for (index, speaker) in meeting.speakers.iter().enumerate() {
            insert_speaker(connection, meeting, index, speaker).await?;
        }
        for (index, segment) in meeting.segments.iter().enumerate() {
            insert_segment(connection, meeting, index, segment).await?;
        }
        Ok(())
    }
}

async fn insert_speaker(
    connection: &mut SqliteConnection,
    meeting: &FixtureMeeting,
    index: usize,
    speaker: &Speaker,
) -> Result<(), DbError> {
    let id = fixture_uuid(&speaker_key(meeting.id.as_str(), &speaker.label));
    let is_me = speaker
        .attendee
        .as_ref()
        .is_some_and(|email| email.as_str() == USER_EMAIL);
    let color = PaletteColor::cycle(index);
    sqlx::query!(
        "INSERT INTO speakers (id, meeting_id, label, person_id, is_me, color)
         VALUES (?1, ?2, ?3, (SELECT id FROM people WHERE email = ?4), ?5, ?6)",
        id,
        meeting.id,
        speaker.label,
        speaker.attendee,
        is_me,
        color,
    )
    .execute(connection)
    .await?;
    Ok(())
}

async fn insert_segment(
    connection: &mut SqliteConnection,
    meeting: &FixtureMeeting,
    index: usize,
    segment: &Segment,
) -> Result<(), DbError> {
    let id = fixture_uuid(&segment_key(meeting.id.as_str(), index));
    let speaker = fixture_uuid(&speaker_key(meeting.id.as_str(), &segment.speaker));
    let words: Json<Vec<Value>> = Json(segment.words.iter().map(word_json).collect());
    sqlx::query!(
        "INSERT INTO transcript_segments (id, meeting_id, channel, speaker_id, start_ms, end_ms, text, words_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        id,
        meeting.id,
        segment.channel,
        speaker,
        segment.start_ms,
        segment.end_ms,
        segment.text,
        words,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// One word as `transcript_segments.words_json` stores it (the shape of
/// `deino_core::segments::WordTiming`).
fn word_json(word: &super::super::model::Word) -> Value {
    json!({ "text": word.text, "start": word.start_ms, "end": word.end_ms })
}
