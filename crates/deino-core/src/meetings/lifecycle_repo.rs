//! The meeting lifecycle port and its SQLite implementation.
//!
//! Every write returns `false` when no meeting has the ID, so callers decide
//! whether that is an error. Each method is one statement on the writer.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{
    CalendarEventId, MeetingId, MeetingSource, MeetingStatus, Seconds, SeriesId, TemplateId,
};

use super::{
    Meeting, MeetingAudio, MeetingLink, MeetingState, MeetingTiming, NewMeeting, SqliteMeetingRepo,
    StatusChange,
};

/// Creates meetings and moves them through their lifecycle.
#[async_trait]
pub trait MeetingLifecycleRepo: Send + Sync {
    /// Inserts a new meeting. Fails if the ID is taken.
    async fn create(&self, meeting: &NewMeeting) -> Result<(), DbError>;

    /// The lifecycle, timing, links and files of the meeting with `id`.
    async fn state(&self, id: &MeetingId) -> Result<Option<MeetingState>, DbError>;

    /// Meetings outside the trash in `status`, newest first. Recovery uses it
    /// to find meetings stuck in `recording`.
    async fn list_by_status(&self, status: MeetingStatus) -> Result<Vec<Meeting>, DbError>;

    /// Applies `change` if the meeting is still in `change.from`.
    ///
    /// Returns `false` when the meeting is missing or its status moved on.
    async fn set_status(&self, id: &MeetingId, change: StatusChange) -> Result<bool, DbError>;

    /// Records when capture started and stopped and how long it recorded.
    async fn set_timing(
        &self,
        id: &MeetingId,
        timing: &MeetingTiming,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Records the paths of the meeting's recording files.
    async fn set_audio(
        &self,
        id: &MeetingId,
        audio: &MeetingAudio,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Links the meeting to a calendar event and series, or unlinks it.
    async fn link(
        &self,
        id: &MeetingId,
        link: &MeetingLink,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Chooses the meeting's note template, or clears it. Fails when no
    /// template has the ID (`meetings.template_id` references `templates`).
    async fn set_template(
        &self,
        id: &MeetingId,
        template: Option<&TemplateId>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;
}

#[async_trait]
impl MeetingLifecycleRepo for SqliteMeetingRepo {
    async fn create(&self, meeting: &NewMeeting) -> Result<(), DbError> {
        let title = meeting.title.as_str();
        sqlx::query!(
            "INSERT INTO meetings (id, title, created_at, updated_at, folder_path, status, source, started_at)
             VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7)",
            meeting.id,
            title,
            meeting.created_at,
            meeting.folder_path,
            meeting.status,
            meeting.source,
            meeting.started_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn state(&self, id: &MeetingId) -> Result<Option<MeetingState>, DbError> {
        let row = sqlx::query_as!(
            StateRow,
            r#"SELECT
                id AS "id!: MeetingId",
                status AS "status: MeetingStatus",
                source AS "source: MeetingSource",
                started_at AS "started_at: DateTime<Utc>",
                ended_at AS "ended_at: DateTime<Utc>",
                duration_s AS "duration: Seconds",
                calendar_event_id AS "calendar_event_id: CalendarEventId",
                series_id AS "series_id: SeriesId",
                template_id AS "template_id: TemplateId",
                deleted_at AS "deleted_at: DateTime<Utc>",
                transcript_deleted_at AS "transcript_deleted_at: DateTime<Utc>",
                audio_path,
                audio_mic_path,
                audio_system_path,
                video_path
            FROM meetings
            WHERE id = ?1"#,
            id,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(row.map(MeetingState::from))
    }

    async fn list_by_status(&self, status: MeetingStatus) -> Result<Vec<Meeting>, DbError> {
        let meetings =
            sqlx::query_file_as!(Meeting, "src/meetings/queries/list_by_status.sql", status)
                .fetch_all(self.database.reader())
                .await?;
        Ok(meetings)
    }

    async fn set_status(&self, id: &MeetingId, change: StatusChange) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE meetings SET status = ?1, updated_at = ?2 WHERE id = ?3 AND status = ?4",
            change.to,
            change.at,
            id,
            change.from,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_timing(
        &self,
        id: &MeetingId,
        timing: &MeetingTiming,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE meetings SET started_at = ?1, ended_at = ?2, duration_s = ?3, updated_at = ?4
             WHERE id = ?5",
            timing.started_at,
            timing.ended_at,
            timing.duration,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_audio(
        &self,
        id: &MeetingId,
        audio: &MeetingAudio,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE meetings
             SET audio_path = ?1, audio_mic_path = ?2, audio_system_path = ?3, video_path = ?4, updated_at = ?5
             WHERE id = ?6",
            audio.audio_path,
            audio.audio_mic_path,
            audio.audio_system_path,
            audio.video_path,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn link(
        &self,
        id: &MeetingId,
        link: &MeetingLink,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE meetings SET calendar_event_id = ?1, series_id = ?2, updated_at = ?3 WHERE id = ?4",
            link.calendar_event_id,
            link.series_id,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_template(
        &self,
        id: &MeetingId,
        template: Option<&TemplateId>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE meetings SET template_id = ?1, updated_at = ?2 WHERE id = ?3",
            template,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// One `meetings` row as [`MeetingLifecycleRepo::state`] reads it.
struct StateRow {
    id: MeetingId,
    status: MeetingStatus,
    source: MeetingSource,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    duration: Option<Seconds>,
    calendar_event_id: Option<CalendarEventId>,
    series_id: Option<SeriesId>,
    template_id: Option<TemplateId>,
    deleted_at: Option<DateTime<Utc>>,
    transcript_deleted_at: Option<DateTime<Utc>>,
    audio_path: Option<String>,
    audio_mic_path: Option<String>,
    audio_system_path: Option<String>,
    video_path: Option<String>,
}

impl From<StateRow> for MeetingState {
    fn from(row: StateRow) -> Self {
        Self {
            id: row.id,
            status: row.status,
            source: row.source,
            started_at: row.started_at,
            ended_at: row.ended_at,
            duration: row.duration,
            calendar_event_id: row.calendar_event_id,
            series_id: row.series_id,
            template_id: row.template_id,
            deleted_at: row.deleted_at,
            transcript_deleted_at: row.transcript_deleted_at,
            audio: MeetingAudio {
                audio_path: row.audio_path,
                audio_mic_path: row.audio_mic_path,
                audio_system_path: row.audio_system_path,
                video_path: row.video_path,
            },
        }
    }
}
