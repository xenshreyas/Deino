//! The automations tables through [`SqliteAutomationRepo`], against the
//! real schema with the fixture meetings seeded.

use deino_db::{Database, test_db};
use deino_domain::{
    AutomationId, AutomationRunId, AutomationRunStatus, AutomationStepStatus, ConditionGroups,
    FakeClock, MeetingId, TagId,
};
use deino_testkit::{at, seed_fixtures};

use super::*;
use crate::testing::query_plan;

const SALES: &str = "meeting-fixture-sales-discovery";
const STANDUP: &str = "meeting-fixture-standup";

async fn seeded() -> SqliteAutomationRepo {
    let database: Database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    SqliteAutomationRepo::new(database)
}

fn automation(name: &str, created: &str) -> Automation {
    let clock = FakeClock::new(at(created));
    Automation {
        id: AutomationId::generate(&clock),
        name: name.into(),
        enabled: true,
        auto_run: true,
        stop_on_failure: false,
        conditions: ConditionGroups::every_meeting(),
        steps: vec![
            Action::Notify {
                message: String::new(),
            },
            Action::RunShell {
                command: "cat > payload.json".into(),
                fields: PayloadFields::default(),
            },
        ],
        created_at: at(created),
        updated_at: at(created),
    }
}

fn request(automation: &Automation, meeting: &str, trigger: Trigger) -> RunRequest {
    RunRequest {
        id: AutomationRunId::generate(&FakeClock::new(at("2026-10-12T09:00:00Z"))),
        automation_id: automation.id,
        meeting_id: MeetingId::new(meeting),
        trigger,
        status: AutomationRunStatus::Pending,
        at: at("2026-10-12T09:00:00Z"),
    }
}

#[tokio::test]
async fn automations_round_trip_in_creation_order_with_unique_names() {
    let repo = seeded().await;
    let later = automation("Later", "2026-10-12T10:00:00Z");
    let first = automation("First", "2026-10-12T09:00:00Z");
    repo.create(&later).await.unwrap();
    repo.create(&first).await.unwrap();

    assert_eq!(repo.list().await.unwrap(), [first.clone(), later.clone()]);
    assert_eq!(repo.get(&first.id).await.unwrap(), Some(first.clone()));
    let clash = automation("FIRST", "2026-10-12T11:00:00Z");
    assert!(repo.create(&clash).await.unwrap_err().is_unique_violation());

    let renamed = Automation {
        name: "Renamed".into(),
        enabled: false,
        ..first.clone()
    };
    assert!(repo.update(&renamed).await.unwrap());
    assert_eq!(repo.get(&first.id).await.unwrap(), Some(renamed));
    assert!(repo.delete(&first.id).await.unwrap());
    assert!(!repo.delete(&first.id).await.unwrap());
    assert_eq!(repo.get(&first.id).await.unwrap(), None);
}

/// Runs `run`'s two steps: the first done, the second timed out.
async fn fail_second_step(repo: &SqliteAutomationRepo, run: AutomationRunId) {
    let steps = [(0, ActionKind::Notify), (1, ActionKind::RunShell)];
    repo.start_steps(&run, &steps).await.unwrap();
    let done = StepResult {
        run_id: run,
        position: 0,
        status: AutomationStepStatus::Done,
        failure: None,
        output: Some(StepOutput::Notified),
        at: at("2026-10-12T09:01:00Z"),
    };
    repo.save_step(&done).await.unwrap();
    let failed = StepResult {
        position: 1,
        status: AutomationStepStatus::Failed,
        failure: Some((StepFailure::Timeout, "too slow".into())),
        output: None,
        ..done
    };
    repo.save_step(&failed).await.unwrap();
    repo.finish(
        &run,
        AutomationRunStatus::Failed,
        at("2026-10-12T09:02:00Z"),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn a_run_keeps_its_steps_until_a_replacing_request_resets_it() {
    let repo = seeded().await;
    let recap = automation("Recap", "2026-10-12T09:00:00Z");
    repo.create(&recap).await.unwrap();
    let first = request(&recap, SALES, Trigger::Pipeline);
    assert!(repo.request(&first, false).await.unwrap());
    fail_second_step(&repo, first.id).await;

    let runs = repo.for_meeting(&MeetingId::new(SALES)).await.unwrap();
    let [run] = runs.as_slice() else {
        panic!("{runs:?}")
    };
    assert_eq!(run.status, AutomationRunStatus::Failed);
    assert_eq!(run.trigger, Trigger::Pipeline);
    assert_eq!(run.steps.len(), 2);
    assert_eq!(run.steps[0].output, Some(StepOutput::Notified));
    assert_eq!(run.steps[1].failure, Some(StepFailure::Timeout));
    assert_eq!(run.steps[1].error.as_deref(), Some("too slow"));

    let again = request(&recap, SALES, Trigger::Manual);
    assert!(
        !repo.request(&again, false).await.unwrap(),
        "kept without replace"
    );
    assert!(repo.request(&again, true).await.unwrap());
    let runs = repo.for_automation(&recap.id).await.unwrap();
    let [run] = runs.as_slice() else {
        panic!("{runs:?}")
    };
    assert_eq!(
        (run.id, run.status, run.trigger),
        (first.id, AutomationRunStatus::Pending, Trigger::Manual)
    );
    assert!(run.steps.is_empty() && run.finished_at.is_none());
}

#[tokio::test]
async fn tag_triggers_keep_their_tag_and_deleting_an_automation_drops_its_runs() {
    let repo = seeded().await;
    let recap = automation("Recap", "2026-10-12T09:00:00Z");
    repo.create(&recap).await.unwrap();
    let tag: TagId = "01920000-0000-7000-8000-000000000009".parse().unwrap();
    sqlx::query("INSERT INTO tags (id, name, color, created_at, updated_at) VALUES (?1, 'Sales', 1, ?2, ?2)")
        .bind(tag)
        .bind(at("2026-10-12T09:00:00Z"))
        .execute(repo.database.writer())
        .await
        .unwrap();
    let armed = RunRequest {
        status: AutomationRunStatus::Armed,
        ..request(&recap, STANDUP, Trigger::TagApplied { tag_id: tag })
    };
    repo.request(&armed, false).await.unwrap();

    let runs = repo.for_meeting(&MeetingId::new(STANDUP)).await.unwrap();
    assert_eq!(runs[0].trigger, Trigger::TagApplied { tag_id: tag });
    assert_eq!(runs[0].status, AutomationRunStatus::Armed);
    repo.delete(&recap.id).await.unwrap();
    assert!(
        repo.for_meeting(&MeetingId::new(STANDUP))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn the_meeting_menu_reads_runs_through_the_meeting_index() {
    let repo = seeded().await;
    let plan = query_plan(
        &repo.database,
        "SELECT id FROM automation_runs WHERE meeting_id = ?1 ORDER BY requested_at DESC, id",
    )
    .await;

    assert!(
        plan.iter()
            .any(|step| step.contains("idx_automation_runs_meeting")),
        "{plan:?}"
    );
}
