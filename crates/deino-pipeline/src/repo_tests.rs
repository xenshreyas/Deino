//! [`SqliteJobRepo`] against the real schema: scheduling, compare-and-set
//! claims and settlements, the attempt history, pruning and query plans.

use chrono::{DateTime, TimeDelta, Utc};
use deino_db::{Database, test_db};
use deino_domain::{JobStatus, MeetingId, RunOutcome, StageId};
use deino_testkit::{at, seed};

use crate::{Job, JobKey, JobRepo, Settlement, SqliteJobRepo};

const NOW: &str = "2026-10-08T09:00:00Z";

async fn repo_with_meetings(ids: &[&str]) -> (SqliteJobRepo, Database) {
    let database = test_db().await;
    for id in ids {
        seed::meeting(&database, id, "Standup", at(NOW)).await;
    }
    (SqliteJobRepo::new(database.clone()), database)
}

fn stages(names: &[&str]) -> Vec<StageId> {
    names.iter().map(|name| StageId::new(*name)).collect()
}

fn key(meeting: &str, stage: &str) -> JobKey {
    JobKey {
        meeting_id: MeetingId::new(meeting),
        stage: StageId::new(stage),
    }
}

fn later(seconds: i64) -> DateTime<Utc> {
    at(NOW) + TimeDelta::seconds(seconds)
}

fn settlement(status: JobStatus, outcome: RunOutcome, attempts: u32) -> Settlement {
    Settlement {
        status,
        outcome,
        attempts,
        not_before: None,
        message: Some("the provider did not answer".into()),
        at: later(5),
    }
}

async fn job(repo: &SqliteJobRepo, meeting: &str, stage: &str) -> Job {
    let jobs = repo.jobs(&MeetingId::new(meeting)).await.unwrap();
    jobs.into_iter()
        .find(|job| job.stage == StageId::new(stage))
        .unwrap()
}

/// `(attempt, outcome)` of every attempt of a job, oldest first.
async fn runs(database: &Database, job: &JobKey) -> Vec<(u32, Option<String>)> {
    sqlx::query_as(
        "SELECT attempt, outcome FROM job_runs WHERE meeting_id = ?1 AND stage = ?2 ORDER BY id",
    )
    .bind(&job.meeting_id)
    .bind(&job.stage)
    .fetch_all(database.reader())
    .await
    .unwrap()
}

async fn query_plan(database: &Database, sql: &str) -> String {
    let explain = format!("EXPLAIN QUERY PLAN {}", sql.replace("?1", "NULL"));
    let steps: Vec<(i64, i64, i64, String)> = sqlx::query_as(&explain)
        .fetch_all(database.reader())
        .await
        .unwrap();
    steps
        .into_iter()
        .map(|(.., detail)| detail)
        .collect::<Vec<_>>()
        .join("; ")
}

#[tokio::test]
async fn scheduling_creates_fresh_pending_jobs() {
    let (repo, _) = repo_with_meetings(&["meeting-a"]).await;

    repo.schedule(&MeetingId::new("meeting-a"), &stages(&["a", "b"]), at(NOW))
        .await
        .unwrap();

    let jobs = repo.jobs(&MeetingId::new("meeting-a")).await.unwrap();
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|job| job.status == JobStatus::Pending
        && job.attempts == 0
        && job.updated_at == at(NOW)));
}

#[tokio::test]
async fn rescheduling_resets_finished_jobs_but_never_a_running_one() {
    let (repo, _) = repo_with_meetings(&["meeting-a"]).await;
    let meeting = MeetingId::new("meeting-a");
    repo.schedule(&meeting, &stages(&["a", "b"]), at(NOW))
        .await
        .unwrap();
    repo.start(&key("meeting-a", "a"), later(1)).await.unwrap();
    let failed = settlement(JobStatus::Failed, RunOutcome::Failed, 1);
    repo.settle(&key("meeting-a", "a"), JobStatus::Running, &failed)
        .await
        .unwrap();
    repo.start(&key("meeting-a", "b"), later(6)).await.unwrap();

    repo.schedule(&meeting, &stages(&["a", "b"]), later(9))
        .await
        .unwrap();

    let reset = job(&repo, "meeting-a", "a").await;
    assert_eq!(
        (reset.status, reset.attempts, reset.message),
        (JobStatus::Pending, 0, None)
    );
    assert_eq!(
        job(&repo, "meeting-a", "b").await.status,
        JobStatus::Running
    );
}

#[tokio::test]
async fn a_pending_job_is_claimed_once_and_opens_an_attempt() {
    let (repo, database) = repo_with_meetings(&["meeting-a"]).await;
    repo.schedule(&MeetingId::new("meeting-a"), &stages(&["a"]), at(NOW))
        .await
        .unwrap();

    let first = repo.start(&key("meeting-a", "a"), later(1)).await.unwrap();
    let second = repo.start(&key("meeting-a", "a"), later(2)).await.unwrap();

    assert_eq!((first, second), (Some(1), None));
    assert_eq!(
        job(&repo, "meeting-a", "a").await.status,
        JobStatus::Running
    );
    assert_eq!(runs(&database, &key("meeting-a", "a")).await, [(1, None)]);
}

#[tokio::test]
async fn settling_applies_only_from_the_expected_status_and_closes_the_attempt() {
    let (repo, database) = repo_with_meetings(&["meeting-a"]).await;
    let job_key = key("meeting-a", "a");
    repo.schedule(&MeetingId::new("meeting-a"), &stages(&["a"]), at(NOW))
        .await
        .unwrap();
    repo.start(&job_key, later(1)).await.unwrap();
    let retry = Settlement {
        not_before: Some(later(15)),
        ..settlement(JobStatus::Pending, RunOutcome::Failed, 1)
    };

    let stale = repo
        .settle(&job_key, JobStatus::Pending, &retry)
        .await
        .unwrap();
    let applied = repo
        .settle(&job_key, JobStatus::Running, &retry)
        .await
        .unwrap();

    assert!(!stale && applied);
    let stored = job(&repo, "meeting-a", "a").await;
    assert_eq!(stored.not_before, Some(later(15)));
    assert_eq!(
        stored.message.as_deref(),
        Some("the provider did not answer")
    );
    repo.start(&job_key, later(15)).await.unwrap();
    assert_eq!(
        runs(&database, &job_key).await,
        [(1, Some("failed".into())), (2, None)]
    );
}

#[tokio::test]
async fn work_and_running_lists_see_only_unfinished_jobs() {
    let (repo, _) = repo_with_meetings(&["meeting-a", "meeting-b", "meeting-c"]).await;
    for meeting in ["meeting-a", "meeting-b", "meeting-c"] {
        repo.schedule(&MeetingId::new(meeting), &stages(&["a"]), at(NOW))
            .await
            .unwrap();
    }
    repo.start(&key("meeting-b", "a"), later(1)).await.unwrap();
    repo.start(&key("meeting-c", "a"), later(1)).await.unwrap();
    let done = settlement(JobStatus::Succeeded, RunOutcome::Succeeded, 1);
    repo.settle(&key("meeting-c", "a"), JobStatus::Running, &done)
        .await
        .unwrap();

    let with_work = repo.meetings_with_work().await.unwrap();
    let running = repo.running().await.unwrap();

    assert_eq!(
        with_work,
        [MeetingId::new("meeting-a"), MeetingId::new("meeting-b")]
    );
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].key(), key("meeting-b", "a"));
}

#[tokio::test]
async fn pruning_deletes_jobs_of_retired_stages_with_their_attempts() {
    let (repo, database) = repo_with_meetings(&["meeting-a"]).await;
    repo.schedule(
        &MeetingId::new("meeting-a"),
        &stages(&["kept", "retired"]),
        at(NOW),
    )
    .await
    .unwrap();
    repo.start(&key("meeting-a", "retired"), later(1))
        .await
        .unwrap();

    let pruned = repo.prune(&stages(&["kept"])).await.unwrap();

    assert_eq!(pruned, 1);
    let left: Vec<StageId> = repo
        .jobs(&MeetingId::new("meeting-a"))
        .await
        .unwrap()
        .into_iter()
        .map(|job| job.stage)
        .collect();
    assert_eq!(left, stages(&["kept"]));
    assert!(
        runs(&database, &key("meeting-a", "retired"))
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn deleting_a_meeting_deletes_its_jobs() {
    let (repo, database) = repo_with_meetings(&["meeting-a"]).await;
    repo.schedule(&MeetingId::new("meeting-a"), &stages(&["a"]), at(NOW))
        .await
        .unwrap();

    sqlx::query("DELETE FROM meetings WHERE id = 'meeting-a'")
        .execute(database.writer())
        .await
        .unwrap();

    assert!(
        repo.jobs(&MeetingId::new("meeting-a"))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn job_queries_use_their_indexes() {
    let (_, database) = repo_with_meetings(&[]).await;

    let jobs = query_plan(&database, include_str!("queries/jobs.sql")).await;
    let with_work = query_plan(&database, include_str!("queries/meetings_with_work.sql")).await;
    let running = query_plan(&database, include_str!("queries/running.sql")).await;

    assert!(
        jobs.contains("USING INDEX sqlite_autoindex_jobs_1"),
        "{jobs}"
    );
    assert!(
        with_work.contains("USING COVERING INDEX idx_jobs_status_meeting_id"),
        "{with_work}"
    );
    assert!(
        running.contains("USING INDEX idx_jobs_status_meeting_id"),
        "{running}"
    );
}
