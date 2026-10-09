//! Restarts: a new runner on the same database resumes what the last one
//! left, whether it quit cleanly or crashed, and leaves no zombie rows.

use std::sync::Arc;

use deino_domain::{JobStatus, MeetingStatus, StageId};
use deino_testkit::at;

use super::harness::{Harness, NOW, Waits, key, outcome, stage, started};
use crate::JobRepo;
use crate::testing::{Activity, Behavior};

#[tokio::test]
async fn quitting_mid_stage_leaves_it_pending_and_the_next_start_finishes_it() {
    let before = Arc::new(Activity::default());
    let first = Harness::new(vec![stage("notes", 50, Behavior::UntilCancelled, &before)]).await;
    let meeting = first.record("meeting-a").await;
    first.wait_for(JobStatus::Running, 1).await;

    first.service.shutdown().await;
    assert_eq!(
        first.stages(&meeting).await,
        [("notes".into(), JobStatus::Pending, 0)]
    );

    let after = Arc::new(Activity::default());
    let stages = vec![stage("notes", 50, Behavior::Succeed, &after)];
    let second = Harness::on(first.database.clone(), stages, Waits::Instant).await;
    second.service.until_idle().await;

    assert_eq!(started(&after), ["notes"]);
    assert_eq!(
        second.runs(&meeting, "notes").await,
        [outcome(1, "interrupted"), outcome(1, "succeeded")]
    );
    assert_eq!(second.status_of(&meeting).await, MeetingStatus::Ready);
}

#[tokio::test]
async fn a_job_a_crash_left_running_is_recovered_and_counts_as_an_attempt() {
    let activity = Arc::new(Activity::default());
    let crashed = Harness::new(Vec::new()).await;
    let meeting = crashed
        .meeting("meeting-a", MeetingStatus::Processing)
        .await;
    let notes = StageId::new("notes");
    crashed
        .jobs
        .schedule(&meeting, &[notes], at(NOW))
        .await
        .unwrap();
    crashed
        .jobs
        .start(&key(&meeting, "notes"), at(NOW))
        .await
        .unwrap();

    let stages = vec![stage("notes", 50, Behavior::Succeed, &activity)];
    let restarted = Harness::on(crashed.database.clone(), stages, Waits::Instant).await;
    restarted.service.until_idle().await;

    assert!(restarted.jobs.running().await.unwrap().is_empty());
    assert_eq!(
        restarted.runs(&meeting, "notes").await,
        [outcome(1, "interrupted"), outcome(2, "succeeded")]
    );
    assert_eq!(restarted.status_of(&meeting).await, MeetingStatus::Ready);
}

#[tokio::test]
async fn pending_jobs_and_unscheduled_processing_meetings_resume_at_start() {
    let activity = Arc::new(Activity::default());
    let before = Harness::new(Vec::new()).await;
    let queued = before.meeting("meeting-a", MeetingStatus::Processing).await;
    let never_enqueued = before.meeting("meeting-b", MeetingStatus::Processing).await;
    before
        .jobs
        .schedule(&queued, &[StageId::new("notes")], at(NOW))
        .await
        .unwrap();

    let stages = vec![stage("notes", 50, Behavior::Succeed, &activity)];
    let after = Harness::on(before.database.clone(), stages, Waits::Instant).await;
    after.service.until_idle().await;

    assert_eq!(activity.starts().len(), 2);
    for meeting in [&queued, &never_enqueued] {
        assert_eq!(after.status_of(meeting).await, MeetingStatus::Ready);
    }
}

#[tokio::test]
async fn jobs_of_a_retired_stage_are_dropped_at_start() {
    let activity = Arc::new(Activity::default());
    let before = Harness::new(Vec::new()).await;
    let meeting = before.meeting("meeting-a", MeetingStatus::Ready).await;
    let retired = [StageId::new("legacy_thing")];
    before
        .jobs
        .schedule(&meeting, &retired, at(NOW))
        .await
        .unwrap();

    let stages = vec![stage("notes", 50, Behavior::Succeed, &activity)];
    let after = Harness::on(before.database.clone(), stages, Waits::Instant).await;
    after.service.until_idle().await;

    assert!(after.jobs.jobs(&meeting).await.unwrap().is_empty());
    assert!(activity.starts().is_empty());
    assert_eq!(after.status_of(&meeting).await, MeetingStatus::Ready);
}

#[tokio::test]
async fn a_clock_set_back_since_the_crash_does_not_block_recovery() {
    let activity = Arc::new(Activity::default());
    let crashed = Harness::new(Vec::new()).await;
    let meeting = crashed
        .meeting("meeting-a", MeetingStatus::Processing)
        .await;
    let notes = [StageId::new("notes")];
    crashed
        .jobs
        .schedule(&meeting, &notes, at(NOW))
        .await
        .unwrap();
    // The attempt started an hour after what the restarted app's clock reads.
    let later = at(NOW) + chrono::TimeDelta::hours(1);
    crashed
        .jobs
        .start(&key(&meeting, "notes"), later)
        .await
        .unwrap();

    let stages = vec![stage("notes", 50, Behavior::Succeed, &activity)];
    let restarted = Harness::on(crashed.database.clone(), stages, Waits::Instant).await;
    restarted.service.until_idle().await;

    assert_eq!(
        restarted.runs(&meeting, "notes").await,
        [outcome(1, "interrupted"), outcome(2, "succeeded")]
    );
    assert_eq!(restarted.status_of(&meeting).await, MeetingStatus::Ready);
}
