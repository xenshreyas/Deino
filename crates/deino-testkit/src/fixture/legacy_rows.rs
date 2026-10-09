//! Seed targets for the tables that exist before FND-07: `meetings` and the
//! legacy `transcripts`, written the way the legacy recording path saves them.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use sqlx::SqliteConnection;

use super::model::{FixtureMeeting, Segment};
use super::seeding::SeedTarget;

/// The meeting row. `created_at` is when recording stopped, which is when the
/// legacy path saves a meeting; re-seeding restores title and timestamps.
pub struct LegacyMeetingRows;

#[async_trait]
impl SeedTarget for LegacyMeetingRows {
    async fn seed(
        &self,
        connection: &mut SqliteConnection,
        meeting: &FixtureMeeting,
    ) -> Result<(), DbError> {
        let saved_at = meeting.ended_at();
        sqlx::query!(
            "INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)
            ON CONFLICT (id) DO UPDATE SET
                title = excluded.title,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at",
            meeting.id,
            meeting.title,
            saved_at,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
}

/// One legacy `transcripts` row per segment, replacing the meeting's old rows.
///
/// The legacy `speaker` column holds the capture channel (`mic`/`system`),
/// `timestamp` the wall-clock time, and the audio columns seconds from the
/// start of the recording. Row IDs are stable: `<meeting id>-segment-<index>`.
pub struct LegacyTranscriptRows;

#[async_trait]
impl SeedTarget for LegacyTranscriptRows {
    async fn seed(
        &self,
        connection: &mut SqliteConnection,
        meeting: &FixtureMeeting,
    ) -> Result<(), DbError> {
        sqlx::query!("DELETE FROM transcripts WHERE meeting_id = ?1", meeting.id)
            .execute(&mut *connection)
            .await?;
        for (index, segment) in meeting.segments.iter().enumerate() {
            let row = TranscriptRow::new(meeting, index, segment);
            row.insert(connection).await?;
        }
        Ok(())
    }
}

/// A legacy `transcripts` row, converted from a fixture segment.
struct TranscriptRow<'a> {
    id: String,
    meeting: &'a FixtureMeeting,
    segment: &'a Segment,
    wall_clock: String,
}

impl<'a> TranscriptRow<'a> {
    fn new(meeting: &'a FixtureMeeting, index: usize, segment: &'a Segment) -> Self {
        let started: DateTime<Utc> = meeting.wall_clock(segment.start_ms);
        Self {
            id: format!("{}-segment-{index:04}", meeting.id),
            meeting,
            segment,
            wall_clock: started.format("%H:%M:%S").to_string(),
        }
    }

    async fn insert(&self, connection: &mut SqliteConnection) -> Result<(), DbError> {
        let segment = self.segment;
        let (start, end) = (segment.start_ms.as_secs_f64(), segment.end_ms.as_secs_f64());
        let duration = end - start;
        let channel = segment.channel.as_str();
        sqlx::query!(
            "INSERT INTO transcripts
                (id, meeting_id, transcript, timestamp, speaker,
                 audio_start_time, audio_end_time, duration)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            self.id,
            self.meeting.id,
            segment.text,
            self.wall_clock,
            channel,
            start,
            end,
            duration,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
}
