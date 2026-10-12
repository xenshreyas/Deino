//! [`AutomationService`]: definitions, the catalogue, manual and
//! tag-triggered runs, the meeting menu and matching meetings.

use deino_domain::{AutomationRunStatus, Condition, ConditionGroups};

use super::harness::{Harness, SALES, STANDUP, add_tag, meeting, notify, sales_tag};
use super::{
    ActionKind, AutomationError, AutomationInput, AutomationRunner, RunRepo, StepAvailability,
    Trigger,
};

fn input(name: &str) -> AutomationInput {
    AutomationInput {
        name: name.into(),
        enabled: true,
        auto_run: true,
        stop_on_failure: false,
        conditions: ConditionGroups::every_meeting(),
        steps: vec![notify()],
    }
}

fn tagged() -> ConditionGroups {
    ConditionGroups(vec![vec![Condition::HasTag {
        tag_id: sales_tag(),
    }]])
}

#[tokio::test]
async fn definitions_are_checked_named_uniquely_and_duplicated_switched_off() {
    let harness = Harness::new(vec![], vec![]).await;
    let service = harness.service();

    let recap = service.create(input(" Recap ")).await.unwrap();
    assert_eq!(recap.name, "Recap");
    assert!(matches!(
        service.create(input("recap")).await,
        Err(AutomationError::DuplicateName)
    ));
    assert!(matches!(
        service.create(input("")).await,
        Err(AutomationError::Invalid { .. })
    ));

    let copy = service.duplicate(&recap.id).await.unwrap();
    assert_eq!((copy.name.as_str(), copy.enabled), ("Recap (copy)", false));
    assert_eq!(copy.steps, recap.steps);
    let renamed = service.update(&copy.id, input("Weekly")).await.unwrap();
    assert_eq!(
        (renamed.name.as_str(), renamed.created_at),
        ("Weekly", copy.created_at)
    );
    service.delete(&copy.id).await.unwrap();
    assert!(matches!(
        service.get(&copy.id).await,
        Err(AutomationError::NotFound(_))
    ));
    assert_eq!(service.list().await.unwrap(), [recap]);
}

#[tokio::test]
async fn the_catalogue_offers_local_steps_and_shows_provider_steps_as_wip() {
    let harness = Harness::new(vec![], vec![]).await;

    let catalog = harness.service().catalog();

    let ready: Vec<ActionKind> = catalog
        .iter()
        .filter(|entry| entry.availability == StepAvailability::Ready)
        .map(|entry| entry.kind)
        .collect();
    assert_eq!(ready, [ActionKind::AddTag, ActionKind::Notify]);
    let wip = catalog
        .iter()
        .find(|entry| entry.kind == ActionKind::Slack)
        .unwrap();
    assert_eq!(wip.availability, StepAvailability::Wip);
    assert_eq!(catalog.len(), 13);
}

#[tokio::test]
async fn running_by_hand_replaces_the_last_run_and_schedules_the_stage() {
    let harness = Harness::new(vec![], vec![]).await;
    let service = harness.service();
    let recap = harness
        .add("Recap", ConditionGroups::every_meeting(), vec![notify()])
        .await;
    harness
        .engine()
        .run_meeting(&meeting(SALES), false)
        .await
        .unwrap();

    service
        .run(&recap.id, &[meeting(SALES), meeting(SALES)])
        .await
        .unwrap();

    let run = &harness.repo.for_meeting(&meeting(SALES)).await.unwrap()[0];
    assert_eq!(
        (run.status, run.trigger),
        (AutomationRunStatus::Pending, Trigger::Manual)
    );
    assert!(run.steps.is_empty());
    assert_eq!(harness.scheduler.scheduled(), [meeting(SALES)]);
    let missing = service.run(&recap.id, &[meeting("meeting-nope")]).await;
    assert!(matches!(missing, Err(AutomationError::MeetingNotFound(_))));
    assert!(service.run(&recap.id, &[]).await.is_err());
}

#[tokio::test]
async fn testing_on_the_latest_meeting_runs_on_the_newest_finished_one() {
    let harness = Harness::new(vec![], vec![]).await;
    let service = harness.service();
    let recap = harness
        .add("Recap", ConditionGroups::every_meeting(), vec![notify()])
        .await;

    let latest = service.test_on_latest(&recap.id).await.unwrap();

    let newest = harness.service().matching(&recap.id).await.unwrap()[0]
        .meeting
        .id
        .clone();
    assert_eq!(latest, newest);
    assert_eq!(harness.scheduler.scheduled(), [latest]);
}

#[tokio::test]
async fn a_tag_on_a_processed_meeting_runs_the_automations_waiting_for_it_once() {
    let harness = Harness::new(vec![], vec![]).await;
    let service = harness.service();
    harness.add("Tagged", tagged(), vec![notify()]).await;
    harness
        .add(
            "Everything",
            ConditionGroups::every_meeting(),
            vec![notify()],
        )
        .await;
    harness.tag_meeting(SALES).await;

    let added = service
        .tag_applied(&meeting(SALES), &sales_tag())
        .await
        .unwrap();
    let again = service
        .tag_applied(&meeting(SALES), &sales_tag())
        .await
        .unwrap();

    assert_eq!((added, again), (1, 0));
    let runs = harness.repo.for_meeting(&meeting(SALES)).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Pending);
    assert_eq!(
        runs[0].trigger,
        Trigger::TagApplied {
            tag_id: sales_tag()
        }
    );
    assert_eq!(harness.scheduler.scheduled(), [meeting(SALES)]);
}

#[tokio::test]
async fn the_meeting_menu_lists_every_automation_matching_ones_first_with_runs() {
    let harness = Harness::new(vec![], vec![]).await;
    let service = harness.service();
    harness.add("Board", tagged(), vec![add_tag()]).await;
    let recap = harness
        .add("Recap", ConditionGroups::every_meeting(), vec![notify()])
        .await;
    harness
        .engine()
        .run_meeting(&meeting(STANDUP), false)
        .await
        .unwrap();

    let menu = service.for_meeting(&meeting(STANDUP)).await.unwrap();

    let names: Vec<(&str, bool)> = menu
        .iter()
        .map(|entry| (entry.automation.name.as_str(), entry.matches))
        .collect();
    assert_eq!(names, [("Recap", true), ("Board", false)]);
    assert_eq!(
        menu[0].run.as_ref().map(|run| run.status),
        Some(AutomationRunStatus::Done)
    );
    assert!(menu[1].run.is_none());
    let matching = service.matching(&recap.id).await.unwrap();
    assert!(
        matching
            .iter()
            .any(|entry| entry.meeting.id == meeting(STANDUP) && entry.run.is_some())
    );
    assert!(
        service
            .matching(&menu[1].automation.id)
            .await
            .unwrap()
            .is_empty()
    );
}
