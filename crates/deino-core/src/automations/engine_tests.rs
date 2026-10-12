//! The `automations` stage's rules: which runs start, step order, failure
//! handling, retries and idempotence.

use deino_domain::{AutomationRunStatus, AutomationStepStatus, Condition, ConditionGroups};

use super::harness::{Harness, SALES, STANDUP, add_tag, meeting, notify, sales_tag};
use super::{AutomationRunner, EngineOutcome, RunRepo, Script, StepFailure, Trigger};

fn titled(text: &str) -> ConditionGroups {
    ConditionGroups(vec![vec![Condition::TitleContains { text: text.into() }]])
}

#[tokio::test]
async fn a_matching_automation_runs_once_per_meeting_however_often_the_stage_runs() {
    let harness = Harness::new(vec![], vec![]).await;
    harness
        .add(
            "Recap",
            ConditionGroups::every_meeting(),
            vec![notify(), add_tag()],
        )
        .await;
    let engine = harness.engine();

    let first = engine.run_meeting(&meeting(SALES), false).await.unwrap();
    let again = engine.run_meeting(&meeting(SALES), false).await.unwrap();

    assert_eq!(first, EngineOutcome::Ran { runs: 1, failed: 0 });
    assert_eq!(again, EngineOutcome::NothingToRun);
    let runs = harness.repo.for_meeting(&meeting(SALES)).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (runs[0].status, runs[0].trigger),
        (AutomationRunStatus::Done, Trigger::Pipeline)
    );
    let statuses: Vec<_> = runs[0].steps.iter().map(|step| step.status).collect();
    assert_eq!(statuses, [AutomationStepStatus::Done; 2]);
    assert_eq!(harness.notify.calls().len(), 1);
    assert_eq!(
        harness.tag.calls(),
        [(meeting(SALES), 1)],
        "the tag step saw the first output"
    );
}

#[tokio::test]
async fn automations_that_are_off_manual_or_unmatched_do_not_run_on_their_own() {
    let harness = Harness::new(vec![], vec![]).await;
    let mut off = harness
        .add("Off", ConditionGroups::every_meeting(), vec![notify()])
        .await;
    off.enabled = false;
    harness.save(&off).await;
    let mut manual = harness
        .add("Manual", ConditionGroups::every_meeting(), vec![notify()])
        .await;
    manual.auto_run = false;
    harness.save(&manual).await;
    harness
        .add("Board only", titled("board"), vec![notify()])
        .await;

    let outcome = harness
        .engine()
        .run_meeting(&meeting(SALES), false)
        .await
        .unwrap();

    assert_eq!(outcome, EngineOutcome::NothingToRun);
    assert!(harness.notify.calls().is_empty());
}

#[tokio::test]
async fn a_failed_step_does_not_block_the_next_unless_the_automation_stops_on_failure() {
    let harness = Harness::new(vec![Script::Fail, Script::Fail], vec![]).await;
    harness
        .add(
            "Keeps going",
            titled("discovery"),
            vec![notify(), add_tag()],
        )
        .await;
    let mut stops = harness
        .add("Stops", titled("standup"), vec![notify(), add_tag()])
        .await;
    stops.stop_on_failure = true;
    harness.save(&stops).await;
    let engine = harness.engine();

    let sales = engine.run_meeting(&meeting(SALES), false).await.unwrap();
    let standup = engine.run_meeting(&meeting(STANDUP), false).await.unwrap();

    assert_eq!(sales, EngineOutcome::Ran { runs: 1, failed: 1 });
    assert_eq!(standup, EngineOutcome::Ran { runs: 1, failed: 1 });
    let kept = &harness.repo.for_meeting(&meeting(SALES)).await.unwrap()[0];
    assert_eq!(kept.status, AutomationRunStatus::Failed);
    assert_eq!(kept.steps[0].failure, Some(StepFailure::MissingTarget));
    assert_eq!(kept.steps[1].status, AutomationStepStatus::Done);
    let stopped = &harness.repo.for_meeting(&meeting(STANDUP)).await.unwrap()[0];
    assert_eq!(stopped.steps[1].status, AutomationStepStatus::Skipped);
    assert_eq!(harness.tag.calls(), [(meeting(SALES), 0)]);
}

#[tokio::test]
async fn a_failure_that_may_pass_retries_the_stage_and_resumes_after_the_done_steps() {
    let harness = Harness::new(vec![Script::FailRetryably, Script::FailRetryably], vec![]).await;
    harness
        .add(
            "Recap",
            ConditionGroups::every_meeting(),
            vec![add_tag(), notify()],
        )
        .await;
    let engine = harness.engine();

    let first = engine.run_meeting(&meeting(SALES), false).await;
    let second = engine.run_meeting(&meeting(SALES), true).await.unwrap();

    assert!(first.unwrap_err().is_retryable());
    assert_eq!(
        second,
        EngineOutcome::Ran { runs: 1, failed: 1 },
        "the last attempt stores it"
    );
    assert_eq!(
        harness.tag.calls().len(),
        1,
        "the done step is not run again"
    );
    assert_eq!(
        harness.notify.calls(),
        [(meeting(SALES), 1), (meeting(SALES), 1)]
    );
    let run = &harness.repo.for_meeting(&meeting(SALES)).await.unwrap()[0];
    assert_eq!(run.steps[1].failure, Some(StepFailure::Storage));
}

/// A harness with "Tagged", an automation that waits for the sales tag.
async fn tagged_harness() -> Harness {
    let harness = Harness::new(vec![], vec![]).await;
    let tagged = ConditionGroups(vec![vec![Condition::HasTag {
        tag_id: sales_tag(),
    }]]);
    harness.add("Tagged", tagged, vec![notify()]).await;
    harness
}

#[tokio::test]
async fn an_armed_run_runs_once_its_meeting_is_processed() {
    let harness = tagged_harness().await;
    harness.set_status(STANDUP, "scheduled").await;
    harness.tag_meeting(STANDUP).await;
    let armed = harness
        .service()
        .tag_applied(&meeting(STANDUP), &sales_tag())
        .await
        .unwrap();
    assert_eq!(armed, 1);
    assert!(
        harness.scheduler.scheduled().is_empty(),
        "an upcoming meeting is armed, not run"
    );

    harness.set_status(STANDUP, "ready").await;
    let outcome = harness
        .engine()
        .run_meeting(&meeting(STANDUP), false)
        .await
        .unwrap();

    assert_eq!(outcome, EngineOutcome::Ran { runs: 1, failed: 0 });
    let run = &harness.repo.for_meeting(&meeting(STANDUP)).await.unwrap()[0];
    let tag_id = sales_tag();
    assert_eq!(run.trigger, Trigger::TagApplied { tag_id });
}

#[tokio::test]
async fn an_armed_run_is_dropped_once_its_conditions_stop_matching() {
    let harness = tagged_harness().await;
    harness.tag_meeting(SALES).await;
    harness.set_status(SALES, "processing").await;
    harness
        .service()
        .tag_applied(&meeting(SALES), &sales_tag())
        .await
        .unwrap();
    sqlx::query("DELETE FROM meeting_tags WHERE meeting_id = ?1")
        .bind(SALES)
        .execute(harness.database.writer())
        .await
        .unwrap();

    let dropped = harness
        .engine()
        .run_meeting(&meeting(SALES), false)
        .await
        .unwrap();

    assert_eq!(dropped, EngineOutcome::NothingToRun);
    let runs = harness.repo.for_meeting(&meeting(SALES)).await.unwrap();
    assert!(runs.is_empty());
}
