//! The speaker port and its SQLite implementation.

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, PaletteColor, PersonId, SpeakerId};

use super::{NewSpeaker, Speaker, SpeakerVoice, SqliteSegmentRepo, voices};

/// Stores the speakers of each meeting's transcript.
#[async_trait]
pub trait SpeakerRepo: Send + Sync {
    /// Adds a speaker. Fails if the label is taken in its meeting.
    async fn add_speaker(&self, speaker: &Speaker) -> Result<(), DbError>;

    /// The meeting's speakers, by label.
    async fn speakers(&self, meeting: &MeetingId) -> Result<Vec<Speaker>, DbError>;

    /// Changes the speaker's label. Fails if the label is taken in its meeting.
    async fn rename_speaker(&self, id: &SpeakerId, label: &str) -> Result<bool, DbError>;

    /// Identifies the speaker as a person, or forgets who it was.
    async fn assign_person(
        &self,
        id: &SpeakerId,
        person: Option<&PersonId>,
    ) -> Result<bool, DbError>;

    /// Removes the speaker; its segments keep their text and lose the speaker.
    async fn remove_speaker(&self, id: &SpeakerId) -> Result<bool, DbError>;

    /// Replaces every speaker of `meeting` except the user's own (`is_me`)
    /// with `speakers`, in one transaction: a new diarization result.
    /// Segments of a removed speaker keep their text and lose the speaker.
    async fn replace_speakers(
        &self,
        meeting: &MeetingId,
        speakers: &[NewSpeaker],
    ) -> Result<(), DbError>;

    /// The voice diarization stored for the speaker; `None` for an unknown
    /// speaker or one without a voice.
    async fn voice(&self, id: &SpeakerId) -> Result<Option<SpeakerVoice>, DbError>;
}

#[async_trait]
impl SpeakerRepo for SqliteSegmentRepo {
    async fn add_speaker(&self, speaker: &Speaker) -> Result<(), DbError> {
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
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn speakers(&self, meeting: &MeetingId) -> Result<Vec<Speaker>, DbError> {
        let speakers = sqlx::query_file_as!(Speaker, "src/segments/queries/speakers.sql", meeting)
            .fetch_all(self.database.reader())
            .await?;
        Ok(speakers)
    }

    async fn rename_speaker(&self, id: &SpeakerId, label: &str) -> Result<bool, DbError> {
        let result = sqlx::query!("UPDATE speakers SET label = ?1 WHERE id = ?2", label, id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn assign_person(
        &self,
        id: &SpeakerId,
        person: Option<&PersonId>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE speakers SET person_id = ?1 WHERE id = ?2",
            person,
            id
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn remove_speaker(&self, id: &SpeakerId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM speakers WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn replace_speakers(
        &self,
        meeting: &MeetingId,
        speakers: &[NewSpeaker],
    ) -> Result<(), DbError> {
        voices::replace(&self.database, meeting, speakers).await
    }

    async fn voice(&self, id: &SpeakerId) -> Result<Option<SpeakerVoice>, DbError> {
        voices::voice(&self.database, id).await
    }
}
