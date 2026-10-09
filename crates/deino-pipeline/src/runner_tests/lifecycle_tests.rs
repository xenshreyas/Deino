//! A meeting's pipeline from `processing` to `ready` or `failed`: order,
//! progress events, retries, failures, re-runs and cancellation.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{JobStatus, MeetingStatus};
use deino_testkit::at;

use super::harness::{Harness, NOW, Waits, outcome, stage, started};
use crate::testing::{Activity, Behavior};

#[tokio::test]
async fn stopping_a_recording_runs_every_stage_in_order_and_readies_the_meeting() {
    let activity = Arc::new(Activity::default());
    let harness = Harness::new(vec![
        stage("index", 80, Behavior::Succeed, &activity),
        stage("finalize_audio", 10, Behavior::Succeed, &activity),
        stage("notes", 50, Behavior::Skip, &activity),
    ])
    .await;

    let meeting = harness.record("meeting-a").await;
    harness.service.until_idle().await;

    assert_eq!(started(&activity), ["finalize_audio", "notes", "index"]);
    assert_eq!(
        harness.stages(&meeting).await,
        [
            ("finalize_audio".into(), JobStatus::Succeeded, 1),
            ("notes".into(), JobStatus::Skipped, 1),
            ("index".into(), JobStatus::Succeeded, 1),
        ]
    );
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Ready);
}

#[tokio::test]
async fn progress_is_reported_in_the_order_it_happens() {
    let activity = Arc::new(Activity::default());
    let harness = Harness::new(vec![
        stage("a", 10, Behavior::Succeed, &activity),
        stage("b", 20, Behavior::Succeed, &activity),
    ])
    .await;

    harness.record("meeting-a").await;
    harness.service.until_idle().await;

    let expected = [
        ("a", JobStatus::Pending),
        ("b", JobStatus::Pending),
        ("a", JobStatus::Running),
        ("a", JobStatus::Succeeded),
        ("b", JobStatus::Running),
        ("b", JobStatus::Succeeded),
    ]
    .map(|(stage, status)| (stage.to_owned(), status));
    assert_eq!(harness.events(), expected);
}

#[tokio::test]
async fn a_stage_that_fails_twice_is_retried_with_backoff_from_the_clock() {
    let activity = Arc::new(Activity::default());
    let flaky = Behavior::Fail {
        times: 2,
        retryable: true,
    };
    let harness = Harness::new(vec![stage("notes", 50, flaky, &activity)]).await;

    let meeting = harness.record("meeting-a").await;
    harness.service.until_idle().await;

    // 10 s after the first failure, then 20 s after the second.
    let first_retry = at(NOW) + TimeDelta::seconds(10);
    assert_eq!(
        harness.sleeper.deadlines(),
        [first_retry, first_retry + TimeDelta::seconds(20)]
    );
    assert_eq!(
        harness.runs(&meeting, "notes").await,
        [outcome(1, "failed"), outcome(2, "failed"), outcome(3, "succeeded")]
    );
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Ready);
}

#[tokio::test]
async fn retries_stop_after_three_attempts_and_fail_the_meeting() {
    let activity = Arc::new(Activity::default());
    let broken = Behavior::Fail {
        times: usize::MAX,
        retryable: true,
    };
    let harness = Harness::new(vec![stage("notes", 50, broken, &activity)]).await;

    let meeting = harness.record("meeting-a").await;
    harness.service.until_idle().await;

    let status = harness.service.status(&meeting).await.unwrap();
    assert_eq!((status[0].status, status[0].attempts), (JobStatus::Failed, 3));
    assert_eq!(status[0].message.as_deref(), Some("the provider did not answer"));
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Failed);
}

#[tokio::test]
async fn a_permanent_failure_stops_later_stages_until_the_stage_is_rerun_alone() {
    let activity = Arc::new(Activity::default());
    let once = Behavior::Fail {
        times: 1,
        retryable: false,
    };
    let harness = Harness::new(vec![
        stage("notes", 50, once, &activity),
        stage("index", 80, Behavior::Succeed, &activity),
    ])
    .await;
    let meeting = harness.record("meeting-a").await;
    harness.service.until_idle().await;
    assert_eq!(started(&activity), ["notes"]);
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Failed);

    let rerun = harness
        .service
        .rerun(&meeting, Some(&deino_domain::StageId::new("notes")))
        .await
        .unwrap();
    harness.service.until_idle().await;

    assert_eq!(rerun[0].status, JobStatus::Pending);
    assert_eq!(started(&activity), ["notes", "notes", "index"]);
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Ready);
}

#[tokio::test]
async fn rerunning_one_stage_of_a_ready_meeting_runs_only_that_stage() {
    let activity = Arc::new(Activity::default());
    let harness = Harness::new(vec![
        stage("notes", 50, Behavior::Succeed, &activity),
        stage("index", 80, Behavior::Succeed, &activity),
    ])
    .await;
    let meeting = harness.record("meeting-a").await;
    harness.service.until_idle().await;

    harness
        .service
        .rerun(&meeting, Some(&deino_domain::StageId::new("index")))
        .await
        .unwrap();
    harness.service.until_idle().await;

    assert_eq!(started(&activity), ["notes", "index", "index"]);
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Ready);
}

#[tokio::test]
async fn rerunning_everything_runs_every_stage_again() {
    let activity = Arc::new(Activity::default());
    let harness = Harness::new(vec![
        stage("notes", 50, Behavior::Succeed, &activity),
        stage("index", 80, Behavior::Succeed, &activity),
    ])
    .await;
    let meeting = harness.record("meeting-a").await;
    harness.service.until_idle().await;

    harness.service.rerun(&meeting, None).await.unwrap();
    harness.service.until_idle().await;

    assert_eq!(started(&activity), ["notes", "index", "notes", "index"]);
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Ready);
}

#[tokio::test]
async fn cancelling_a_hung_stage_cancels_it_and_fails_the_meeting() {
    let activity = Arc::new(Activity::default());
    let harness = Harness::new(vec![
        stage("notes", 50, Behavior::Hang, &activity),
        stage("index", 80, Behavior::Succeed, &activity),
    ])
    .await;
    let meeting = harness.record("meeting-a").await;
    harness.wait_for(JobStatus::Running, 1).await;

    assert!(harness.service.cancel(&meeting));
    harness.service.until_idle().await;

    assert_eq!(
        harness.stages(&meeting).await,
        [
            ("notes".into(), JobStatus::Cancelled, 1),
            ("index".into(), JobStatus::Pending, 0),
        ]
    );
    assert_eq!(harness.runs(&meeting, "notes").await, [outcome(1, "cancelled")]);
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Failed);
}

#[tokio::test]
async fn cancelling_during_a_backoff_cancels_the_waiting_stage() {
    let activity = Arc::new(Activity::default());
    let broken = Behavior::Fail {
        times: usize::MAX,
        retryable: true,
    };
    let stages = vec![stage("notes", 50, broken, &activity)];
    let harness = Harness::on(deino_db::test_db().await, stages, Waits::Stalled).await;
    let meeting = harness.record("meeting-a").await;
    // Scheduled, run once, then pending again for the retry.
    harness.wait_for(JobStatus::Pending, 2).await;

    assert!(harness.service.cancel(&meeting));
    harness.service.until_idle().await;

    assert_eq!(
        harness.stages(&meeting).await,
        [("notes".into(), JobStatus::Cancelled, 1)]
    );
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Failed);
}
