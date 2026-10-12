//! Reading runs with their steps: one query for the runs and one for all
//! their steps, joined in memory, so a list never costs a query per run.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{
    AutomationId, AutomationRunId, AutomationRunStatus, AutomationStepStatus, AutomationTrigger,
    MeetingId, TagId,
};
use sqlx::types::Json;

use super::{ActionKind, AutomationRun, RunStep, StepFailure, StepOutput, Trigger};

/// Whose runs to read.
#[derive(Clone, Copy, Debug)]
pub(super) enum RunFilter<'a> {
    /// Every automation's run on a meeting.
    Meeting(&'a MeetingId),
    /// One automation's run on every meeting.
    Automation(&'a AutomationId),
}

/// A row of `automation_runs`.
struct RunRow {
    id: AutomationRunId,
    automation_id: AutomationId,
    meeting_id: MeetingId,
    trigger: AutomationTrigger,
    trigger_tag_id: Option<TagId>,
    status: AutomationRunStatus,
    requested_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
}

/// A row of `automation_run_steps`.
struct StepRow {
    run_id: AutomationRunId,
    position: i64,
    kind: ActionKind,
    status: AutomationStepStatus,
    error_code: Option<StepFailure>,
    error: Option<String>,
    output: Option<Json<StepOutput>>,
    finished_at: Option<DateTime<Utc>>,
}

impl StepRow {
    fn into_step(self) -> RunStep {
        RunStep {
            position: u32::try_from(self.position).unwrap_or(u32::MAX),
            kind: self.kind,
            status: self.status,
            failure: self.error_code,
            error: self.error,
            output: self.output.map(|output| output.0),
            finished_at: self.finished_at,
        }
    }
}

/// The runs `filter` selects, newest request first, each with its steps.
pub(super) async fn load_runs(
    database: &Database,
    filter: RunFilter<'_>,
) -> Result<Vec<AutomationRun>, DbError> {
    let (runs, steps) = match filter {
        RunFilter::Meeting(meeting) => tokio::try_join!(
            runs_of_meeting(database, meeting),
            steps_of_meeting(database, meeting)
        )?,
        RunFilter::Automation(automation) => tokio::try_join!(
            runs_of_automation(database, automation),
            steps_of_automation(database, automation)
        )?,
    };
    let mut by_run: HashMap<AutomationRunId, Vec<RunStep>> = HashMap::new();
    for step in steps {
        by_run
            .entry(step.run_id)
            .or_default()
            .push(step.into_step());
    }
    Ok(runs
        .into_iter()
        .map(|row| AutomationRun {
            steps: by_run.remove(&row.id).unwrap_or_default(),
            id: row.id,
            automation_id: row.automation_id,
            meeting_id: row.meeting_id,
            trigger: Trigger::stored(row.trigger, row.trigger_tag_id),
            status: row.status,
            requested_at: row.requested_at,
            finished_at: row.finished_at,
        })
        .collect())
}

async fn runs_of_meeting(database: &Database, meeting: &MeetingId) -> Result<Vec<RunRow>, DbError> {
    Ok(sqlx::query_as!(
        RunRow,
        r#"SELECT id AS "id: AutomationRunId", automation_id AS "automation_id: AutomationId",
               meeting_id AS "meeting_id: MeetingId", trigger AS "trigger: AutomationTrigger",
               trigger_tag_id AS "trigger_tag_id: TagId", status AS "status: AutomationRunStatus",
               requested_at AS "requested_at: DateTime<Utc>",
               finished_at AS "finished_at: DateTime<Utc>"
           FROM automation_runs WHERE meeting_id = ?1 ORDER BY requested_at DESC, id"#,
        meeting
    )
    .fetch_all(database.reader())
    .await?)
}

async fn runs_of_automation(
    database: &Database,
    automation: &AutomationId,
) -> Result<Vec<RunRow>, DbError> {
    Ok(sqlx::query_as!(
        RunRow,
        r#"SELECT id AS "id: AutomationRunId", automation_id AS "automation_id: AutomationId",
               meeting_id AS "meeting_id: MeetingId", trigger AS "trigger: AutomationTrigger",
               trigger_tag_id AS "trigger_tag_id: TagId", status AS "status: AutomationRunStatus",
               requested_at AS "requested_at: DateTime<Utc>",
               finished_at AS "finished_at: DateTime<Utc>"
           FROM automation_runs WHERE automation_id = ?1 ORDER BY requested_at DESC, id"#,
        automation
    )
    .fetch_all(database.reader())
    .await?)
}

async fn steps_of_meeting(
    database: &Database,
    meeting: &MeetingId,
) -> Result<Vec<StepRow>, DbError> {
    Ok(sqlx::query_as!(
        StepRow,
        r#"SELECT s.run_id AS "run_id: AutomationRunId", s.position, s.kind AS "kind: ActionKind",
               s.status AS "status: AutomationStepStatus",
               s.error_code AS "error_code: StepFailure", s.error,
               s.output_json AS "output: Json<StepOutput>",
               s.finished_at AS "finished_at: DateTime<Utc>"
           FROM automation_run_steps AS s JOIN automation_runs AS r ON r.id = s.run_id
           WHERE r.meeting_id = ?1 ORDER BY s.run_id, s.position"#,
        meeting
    )
    .fetch_all(database.reader())
    .await?)
}

async fn steps_of_automation(
    database: &Database,
    automation: &AutomationId,
) -> Result<Vec<StepRow>, DbError> {
    Ok(sqlx::query_as!(
        StepRow,
        r#"SELECT s.run_id AS "run_id: AutomationRunId", s.position, s.kind AS "kind: ActionKind",
               s.status AS "status: AutomationStepStatus",
               s.error_code AS "error_code: StepFailure", s.error,
               s.output_json AS "output: Json<StepOutput>",
               s.finished_at AS "finished_at: DateTime<Utc>"
           FROM automation_run_steps AS s JOIN automation_runs AS r ON r.id = s.run_id
           WHERE r.automation_id = ?1 ORDER BY s.run_id, s.position"#,
        automation
    )
    .fetch_all(database.reader())
    .await?)
}
