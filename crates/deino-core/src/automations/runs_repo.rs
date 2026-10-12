//! [`RunRepo`]: automations' runs and their steps, on [`SqliteAutomationRepo`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{AutomationId, AutomationRunId, AutomationRunStatus, MeetingId};

use super::rows::{RunFilter, load_runs};
use super::{ActionKind, AutomationRun, RunRequest, SqliteAutomationRepo, StepResult};

/// Reads and writes runs.
#[async_trait]
pub trait RunRepo: Send + Sync {
    /// The latest run of every automation on `meeting`, with steps.
    async fn for_meeting(&self, meeting: &MeetingId) -> Result<Vec<AutomationRun>, DbError>;

    /// The latest run of `automation` on every meeting, with steps.
    async fn for_automation(
        &self,
        automation: &AutomationId,
    ) -> Result<Vec<AutomationRun>, DbError>;

    /// Stores `request` as the run of its automation on its meeting.
    /// `replace` resets an existing run (and drops its steps); otherwise an
    /// existing run is kept. True when the request was stored.
    async fn request(&self, request: &RunRequest, replace: bool) -> Result<bool, DbError>;

    /// Adds a pending row for each step that has none yet.
    async fn start_steps(
        &self,
        run: &AutomationRunId,
        steps: &[(u32, ActionKind)],
    ) -> Result<(), DbError>;

    /// Stores one step's result.
    async fn save_step(&self, result: &StepResult) -> Result<(), DbError>;

    /// Marks the run done or failed.
    async fn finish(
        &self,
        run: &AutomationRunId,
        status: AutomationRunStatus,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// Deletes a run and its steps.
    async fn discard(&self, run: &AutomationRunId) -> Result<(), DbError>;
}

#[async_trait]
impl RunRepo for SqliteAutomationRepo {
    async fn for_meeting(&self, meeting: &MeetingId) -> Result<Vec<AutomationRun>, DbError> {
        load_runs(&self.database, RunFilter::Meeting(meeting)).await
    }

    async fn for_automation(
        &self,
        automation: &AutomationId,
    ) -> Result<Vec<AutomationRun>, DbError> {
        load_runs(&self.database, RunFilter::Automation(automation)).await
    }

    async fn request(&self, request: &RunRequest, replace: bool) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let kind = request.trigger.kind();
        let tag = request.trigger.tag();
        let stored = sqlx::query!(
            "INSERT INTO automation_runs (id, automation_id, meeting_id, trigger, trigger_tag_id,
                 status, requested_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT (automation_id, meeting_id) DO UPDATE SET
                 trigger = excluded.trigger, trigger_tag_id = excluded.trigger_tag_id,
                 status = excluded.status, requested_at = excluded.requested_at,
                 finished_at = NULL, updated_at = excluded.updated_at
             WHERE ?8",
            request.id,
            request.automation_id,
            request.meeting_id,
            kind,
            tag,
            request.status,
            request.at,
            replace,
        )
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            > 0;
        if stored && replace {
            sqlx::query!(
                "DELETE FROM automation_run_steps WHERE run_id =
                     (SELECT id FROM automation_runs WHERE automation_id = ?1 AND meeting_id = ?2)",
                request.automation_id,
                request.meeting_id,
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(stored)
    }

    async fn start_steps(
        &self,
        run: &AutomationRunId,
        steps: &[(u32, ActionKind)],
    ) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for (position, kind) in steps {
            sqlx::query!(
                "INSERT OR IGNORE INTO automation_run_steps (run_id, position, kind, status)
                 VALUES (?1, ?2, ?3, 'pending')",
                run,
                position,
                kind,
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn save_step(&self, result: &StepResult) -> Result<(), DbError> {
        let (failure, error) = result
            .failure
            .clone()
            .map_or((None, None), |(failure, error)| {
                (Some(failure), Some(error))
            });
        let output = result.output.as_ref().map(sqlx::types::Json);
        sqlx::query!(
            "UPDATE automation_run_steps SET status = ?1, error_code = ?2, error = ?3,
                 output_json = ?4, finished_at = ?5
             WHERE run_id = ?6 AND position = ?7",
            result.status,
            failure,
            error,
            output,
            result.at,
            result.run_id,
            result.position,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn finish(
        &self,
        run: &AutomationRunId,
        status: AutomationRunStatus,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE automation_runs SET status = ?1, finished_at = ?2, updated_at = ?2
             WHERE id = ?3",
            status,
            at,
            run,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn discard(&self, run: &AutomationRunId) -> Result<(), DbError> {
        sqlx::query!("DELETE FROM automation_runs WHERE id = ?1", run)
            .execute(self.database.writer())
            .await?;
        Ok(())
    }
}
