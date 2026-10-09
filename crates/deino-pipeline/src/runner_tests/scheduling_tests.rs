//! Concurrency limits and the request API: one stage at a time per
//! meeting, at most two stages across meetings, and the errors `rerun`
//! returns before touching anything.

use std::sync::Arc;

use deino_domain::{ErrorClass, ErrorCode, JobStatus, MeetingId, MeetingStatus, StageId};
use tokio::sync::Semaphore;

use super::harness::{Harness, stage, started};
use crate::MAX_CONCURRENT_STAGES;
use crate::testing::{Activity, Behavior};

#[tokio::test]
async fn at_most_two_stages_run_at_once_across_meetings() {
    let activity = Arc::new(Activity::default());
    let gate = Arc::new(Semaphore::new(0));
    let harness = Harness::new(vec![stage(
        "notes",
        50,
        Behavior::Gated(gate.clone()),
        &activity,
    )])
    .await;
    let meetings = [
        harness.record("meeting-a").await,
        harness.record("meeting-b").await,
        harness.record("meeting-c").await,
    ];
    harness.wait_for(JobStatus::Running, MAX_CONCURRENT_STAGES).await;

    let running_before_release = activity.starts().len();
    gate.add_permits(meetings.len());
    harness.service.until_idle().await;

    assert_eq!(running_before_release, MAX_CONCURRENT_STAGES);
    assert_eq!(activity.peak(), MAX_CONCURRENT_STAGES);
    for meeting in &meetings {
        assert_eq!(harness.status_of(meeting).await, MeetingStatus::Ready);
    }
}

#[tokio::test]
async fn a_meetings_stages_never_overlap_even_when_it_is_woken_again() {
    let activity = Arc::new(Activity::default());
    let gate = Arc::new(Semaphore::new(0));
    let harness = Harness::new(vec![
        stage("notes", 50, Behavior::Gated(gate.clone()), &activity),
        stage("index", 80, Behavior::Succeed, &activity),
    ])
    .await;
    let meeting = harness.record("meeting-a").await;
    harness.wait_for(JobStatus::Running, 1).await;

    let enqueued_again = harness.service.enqueue(&meeting).await.unwrap();
    gate.add_permits(1);
    harness.service.until_idle().await;

    assert!(!enqueued_again, "the running pipeline is not scheduled twice");
    assert_eq!(activity.peak(), 1);
    assert_eq!(started(&activity), ["notes", "index"]);
}

#[tokio::test]
async fn a_status_lists_stages_in_order_with_their_messages() {
    let activity = Arc::new(Activity::default());
    let harness = Harness::new(vec![
        stage("index", 80, Behavior::Succeed, &activity),
        stage("notes", 50, Behavior::Skip, &activity),
    ])
    .await;
    let meeting = harness.record("meeting-a").await;
    harness.service.until_idle().await;

    let status = harness.service.status(&meeting).await.unwrap();

    let stages: Vec<&str> = status.iter().map(|stage| stage.stage.as_str()).collect();
    assert_eq!(stages, ["notes", "index"]);
    assert_eq!(status[0].message.as_deref(), Some("nothing to do"));
    assert!(harness
        .service
        .status(&MeetingId::new("meeting-x"))
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn rerun_refuses_unknown_meetings_and_stages_and_unfinished_recordings() {
    let activity = Arc::new(Activity::default());
    let harness = Harness::new(vec![stage("notes", 50, Behavior::Succeed, &activity)]).await;
    let recording = harness.meeting("meeting-a", MeetingStatus::Recording).await;
    let ready = harness.meeting("meeting-b", MeetingStatus::Ready).await;

    let missing = harness.service.rerun(&MeetingId::new("meeting-x"), None).await;
    let unknown = harness
        .service
        .rerun(&ready, Some(&StageId::new("translate")))
        .await;
    let still_recording = harness.service.rerun(&recording, None).await;

    let code = |result: Result<_, crate::PipelineError>| result.map(|_: Vec<_>| ()).unwrap_err().code();
    assert_eq!(code(missing), ErrorCode::NotFound);
    assert_eq!(code(unknown), ErrorCode::Invalid);
    assert_eq!(code(still_recording), ErrorCode::Conflict);
    assert!(harness.service.status(&recording).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_legacy_ready_meeting_can_be_processed_for_the_first_time() {
    let activity = Arc::new(Activity::default());
    let harness = Harness::new(vec![stage("notes", 50, Behavior::Succeed, &activity)]).await;
    let meeting = harness.meeting("meeting-a", MeetingStatus::Ready).await;

    harness.service.rerun(&meeting, None).await.unwrap();
    harness.service.until_idle().await;

    assert_eq!(
        harness.stages(&meeting).await,
        [("notes".into(), JobStatus::Succeeded, 1)]
    );
    assert_eq!(harness.status_of(&meeting).await, MeetingStatus::Ready);
}
