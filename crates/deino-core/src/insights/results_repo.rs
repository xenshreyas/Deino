//! [`InsightResultRepo`]: insight runs in `insight_results`, and
//! [`SpeakerNames`], the current names behind cited segments.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{InsightId, InsightStatus, MeetingId, SegmentId};
use sqlx::types::Json;

use super::{InsightAnswer, InsightRun, MeetingRun, RunAnswers, RunProgress, SqliteInsightRepo};

/// Reads and writes insight runs.
#[async_trait]
pub trait InsightResultRepo: Send + Sync {
    /// Marks `insight` pending on each of `meetings`, keeping earlier
    /// answers, in one transaction.
    async fn request(
        &self,
        insight: &InsightId,
        meetings: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// The runs on `meeting`.
    async fn for_meeting(&self, meeting: &MeetingId) -> Result<Vec<InsightRun>, DbError>;

    /// The runs of `insight` on meetings outside the trash, newest first.
    async fn for_insight(&self, insight: &InsightId) -> Result<Vec<MeetingRun>, DbError>;

    /// Stores a successful run's answers.
    async fn save_answers(&self, answers: &RunAnswers) -> Result<(), DbError>;

    /// Records that `insight` failed on `meeting` with a user-safe `message`.
    async fn save_failure(
        &self,
        insight: &InsightId,
        meeting: &MeetingId,
        message: &str,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// How many of `insight`'s runs on meetings outside the trash are in
    /// each state.
    async fn progress(&self, insight: &InsightId) -> Result<RunProgress, DbError>;
}

/// The current speaker names of transcript segments.
#[async_trait]
pub trait SpeakerNames: Send + Sync {
    /// The name of each segment of `segments` that still exists.
    async fn names(&self, segments: &[SegmentId]) -> Result<HashMap<SegmentId, String>, DbError>;
}

/// A row of `insight_results`.
struct RunRow {
    insight_id: InsightId,
    meeting_id: MeetingId,
    status: InsightStatus,
    answers: Json<Vec<InsightAnswer>>,
    revision: Option<i64>,
    error: Option<String>,
    requested_at: DateTime<Utc>,
    generated_at: Option<DateTime<Utc>>,
}

impl RunRow {
    fn into_run(self) -> InsightRun {
        InsightRun {
            insight_id: self.insight_id,
            meeting_id: self.meeting_id,
            status: self.status,
            answers: self.answers.0,
            revision: self
                .revision
                .and_then(|revision| u32::try_from(revision).ok()),
            error: self.error,
            requested_at: self.requested_at,
            generated_at: self.generated_at,
        }
    }
}

#[async_trait]
impl InsightResultRepo for SqliteInsightRepo {
    async fn request(
        &self,
        insight: &InsightId,
        meetings: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for meeting in meetings {
            sqlx::query!(
                "INSERT INTO insight_results (insight_id, meeting_id, status, requested_at, updated_at)
                 VALUES (?1, ?2, 'pending', ?3, ?3)
                 ON CONFLICT (insight_id, meeting_id) DO UPDATE SET
                     status = 'pending', error = NULL,
                     requested_at = excluded.requested_at, updated_at = excluded.updated_at",
                insight,
                meeting,
                at,
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn for_meeting(&self, meeting: &MeetingId) -> Result<Vec<InsightRun>, DbError> {
        let rows =
            sqlx::query_file_as!(RunRow, "src/insights/queries/runs_of_meeting.sql", meeting)
                .fetch_all(self.database.reader())
                .await?;
        Ok(rows.into_iter().map(RunRow::into_run).collect())
    }

    async fn for_insight(&self, insight: &InsightId) -> Result<Vec<MeetingRun>, DbError> {
        let rows = sqlx::query_file!("src/insights/queries/runs_of_insight.sql", insight)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| MeetingRun {
                meeting_title: row.meeting_title,
                meeting_started_at: row.meeting_started_at,
                run: RunRow {
                    insight_id: row.insight_id,
                    meeting_id: row.meeting_id,
                    status: row.status,
                    answers: row.answers,
                    revision: row.revision,
                    error: row.error,
                    requested_at: row.requested_at,
                    generated_at: row.generated_at,
                }
                .into_run(),
            })
            .collect())
    }

    async fn save_answers(&self, answers: &RunAnswers) -> Result<(), DbError> {
        let instances = Json(&answers.answers);
        let revision = i64::from(answers.revision);
        sqlx::query!(
            "INSERT INTO insight_results (insight_id, meeting_id, status, instances_json, revision,
                 requested_at, generated_at, updated_at)
             VALUES (?1, ?2, 'done', ?3, ?4, ?5, ?5, ?5)
             ON CONFLICT (insight_id, meeting_id) DO UPDATE SET
                 status = 'done', instances_json = excluded.instances_json,
                 revision = excluded.revision, error = NULL,
                 generated_at = excluded.generated_at, updated_at = excluded.updated_at",
            answers.insight_id,
            answers.meeting_id,
            instances,
            revision,
            answers.at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn save_failure(
        &self,
        insight: &InsightId,
        meeting: &MeetingId,
        message: &str,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO insight_results (insight_id, meeting_id, status, error, requested_at, updated_at)
             VALUES (?1, ?2, 'failed', ?3, ?4, ?4)
             ON CONFLICT (insight_id, meeting_id) DO UPDATE SET
                 status = 'failed', error = excluded.error, updated_at = excluded.updated_at",
            insight,
            meeting,
            message,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn progress(&self, insight: &InsightId) -> Result<RunProgress, DbError> {
        let rows = sqlx::query!(
            r#"SELECT r.status AS "status: InsightStatus", count(*) AS "count!: i64"
               FROM insight_results AS r JOIN meetings AS m ON m.id = r.meeting_id
               WHERE r.insight_id = ?1 AND m.deleted_at IS NULL
               GROUP BY r.status"#,
            insight,
        )
        .fetch_all(self.database.reader())
        .await?;
        let mut progress = RunProgress::default();
        for row in rows {
            let count = u32::try_from(row.count).unwrap_or(u32::MAX);
            match row.status {
                InsightStatus::Pending => progress.pending = count,
                InsightStatus::Done => progress.done = count,
                InsightStatus::Failed => progress.failed = count,
            }
        }
        Ok(progress)
    }
}

#[async_trait]
impl SpeakerNames for SqliteInsightRepo {
    async fn names(&self, segments: &[SegmentId]) -> Result<HashMap<SegmentId, String>, DbError> {
        if segments.is_empty() {
            return Ok(HashMap::new());
        }
        let ids = Json(segments);
        let rows = sqlx::query_file!("src/insights/queries/speaker_names.sql", ids)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.segment_id, row.name))
            .collect())
    }
}

#[cfg(test)]
#[path = "results_repo_tests.rs"]
mod tests;
