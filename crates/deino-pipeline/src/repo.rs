//! The pipeline queue port and its SQLite implementation.
//!
//! Each method is one unit of work on the writer (reads on the read pool).
//! Status changes are compare-and-set, so a job only moves from the state
//! its writer saw: `start` claims a pending job, `settle` ends a running
//! (or abandons a pending) one, and `schedule` never touches a running job.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{JobStatus, MeetingId, StageId};
use sqlx::SqliteConnection;
use sqlx::types::Json;

use crate::{Job, JobKey, Settlement};

/// Where pipeline jobs and their attempts are stored.
#[async_trait]
pub trait JobRepo: Send + Sync {
    /// The meeting's jobs, in no particular order.
    async fn jobs(&self, meeting: &MeetingId) -> Result<Vec<Job>, DbError>;

    /// Meetings with a pending or running job.
    async fn meetings_with_work(&self) -> Result<Vec<MeetingId>, DbError>;

    /// Jobs marked running. At startup, a crash left them so.
    async fn running(&self) -> Result<Vec<Job>, DbError>;

    /// Makes each of `stages` a fresh pending job of `meeting` (no attempts,
    /// no message), creating the rows that are missing. A running job is
    /// left alone, so its attempt can settle.
    async fn schedule(
        &self,
        meeting: &MeetingId,
        stages: &[StageId],
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// Claims a pending job: marks it running, counts the attempt and opens
    /// its `job_runs` row. Returns the attempt number, or `None` when the
    /// job is not pending.
    async fn start(&self, job: &JobKey, at: DateTime<Utc>) -> Result<Option<u32>, DbError>;

    /// Applies `settlement` if the job is still in `from`, closing its open
    /// attempt with the settlement's outcome. Returns whether it applied.
    async fn settle(
        &self,
        job: &JobKey,
        from: JobStatus,
        settlement: &Settlement,
    ) -> Result<bool, DbError>;

    /// Deletes the jobs of every stage not in `registered`, with their
    /// attempts. Returns how many jobs it deleted.
    async fn prune(&self, registered: &[StageId]) -> Result<u64, DbError>;
}

/// [`JobRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteJobRepo {
    database: Database,
}

impl SqliteJobRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl JobRepo for SqliteJobRepo {
    async fn jobs(&self, meeting: &MeetingId) -> Result<Vec<Job>, DbError> {
        // Query files, so the EXPLAIN QUERY PLAN tests check this exact text.
        let jobs = sqlx::query_file_as!(Job, "src/queries/jobs.sql", meeting)
            .fetch_all(self.database.reader())
            .await?;
        Ok(jobs)
    }

    async fn meetings_with_work(&self) -> Result<Vec<MeetingId>, DbError> {
        let meetings = sqlx::query_file_scalar!("src/queries/meetings_with_work.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(meetings)
    }

    async fn running(&self) -> Result<Vec<Job>, DbError> {
        let jobs = sqlx::query_file_as!(Job, "src/queries/running.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(jobs)
    }

    async fn schedule(
        &self,
        meeting: &MeetingId,
        stages: &[StageId],
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for stage in stages {
            sqlx::query!(
                "INSERT INTO jobs (meeting_id, stage, status, attempts, created_at, updated_at)
                 VALUES (?1, ?2, 'pending', 0, ?3, ?3)
                 ON CONFLICT (meeting_id, stage) DO UPDATE SET
                     status = 'pending', attempts = 0, not_before = NULL, message = NULL,
                     updated_at = excluded.updated_at
                 WHERE jobs.status <> 'running'",
                meeting,
                stage,
                at,
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn start(&self, job: &JobKey, at: DateTime<Utc>) -> Result<Option<u32>, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let attempt = sqlx::query_scalar!(
            r#"UPDATE jobs SET status = 'running', attempts = attempts + 1, not_before = NULL, updated_at = ?3
               WHERE meeting_id = ?1 AND stage = ?2 AND status = 'pending'
               RETURNING attempts AS "attempts: u32""#,
            job.meeting_id,
            job.stage,
            at,
        )
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(attempt) = attempt {
            open_run(&mut transaction, job, attempt, at).await?;
        }
        transaction.commit().await?;
        Ok(attempt)
    }

    async fn settle(
        &self,
        job: &JobKey,
        from: JobStatus,
        settlement: &Settlement,
    ) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let updated = sqlx::query!(
            "UPDATE jobs SET status = ?3, attempts = ?4, not_before = ?5, message = ?6, updated_at = ?7
             WHERE meeting_id = ?1 AND stage = ?2 AND status = ?8",
            job.meeting_id,
            job.stage,
            settlement.status,
            settlement.attempts,
            settlement.not_before,
            settlement.message,
            settlement.at,
            from,
        )
        .execute(&mut *transaction)
        .await?;
        let applied = updated.rows_affected() > 0;
        if applied {
            close_run(&mut transaction, job, settlement).await?;
        }
        transaction.commit().await?;
        Ok(applied)
    }

    async fn prune(&self, registered: &[StageId]) -> Result<u64, DbError> {
        // A JSON array parameter keeps the statement fixed for any number of stages.
        let registered = Json(registered);
        let deleted = sqlx::query!(
            "DELETE FROM jobs WHERE stage NOT IN (SELECT value FROM json_each(?1))",
            registered,
        )
        .execute(self.database.writer())
        .await?;
        Ok(deleted.rows_affected())
    }
}

/// Opens the `job_runs` row of a just-started attempt.
async fn open_run(
    connection: &mut SqliteConnection,
    job: &JobKey,
    attempt: u32,
    at: DateTime<Utc>,
) -> Result<(), DbError> {
    sqlx::query!(
        "INSERT INTO job_runs (meeting_id, stage, attempt, started_at) VALUES (?1, ?2, ?3, ?4)",
        job.meeting_id,
        job.stage,
        attempt,
        at,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Closes the job's open attempt, if it has one, with the settlement's outcome.
async fn close_run(
    connection: &mut SqliteConnection,
    job: &JobKey,
    settlement: &Settlement,
) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE job_runs SET finished_at = ?3, outcome = ?4, message = ?5
         WHERE meeting_id = ?1 AND stage = ?2 AND finished_at IS NULL",
        job.meeting_id,
        job.stage,
        settlement.at,
        settlement.outcome,
        settlement.message,
    )
    .execute(connection)
    .await?;
    Ok(())
}
