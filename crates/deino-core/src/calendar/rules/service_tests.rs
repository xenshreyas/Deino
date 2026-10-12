//! The auto-record service over the real schema: plans, "just this one",
//! "all occurrences", and clearing a choice.

use std::sync::Arc;

use deino_domain::{CalendarEventId, Clock};

use super::{AutoRecordDeps, AutoRecordService, ChoiceScope, RecordChoice, RecordReason};
use crate::calendar::TimeRange;
use crate::calendar::test_calendar::{SyncedCalendar, synced_calendar};
use crate::calendar::test_events::{clock, hours, meeting};
use crate::settings::SettingsService;

struct Harness {
    rules: AutoRecordService,
    settings: Arc<SettingsService>,
}

/// Two occurrences of a weekly external meeting (in 1 h and 25 h) and a
/// one-off in 3 h, all organised by a colleague and accepted.
async fn harness() -> Harness {
    let mut weekly = [meeting("weekly-1", 1), meeting("weekly-2", 25)];
    for occurrence in &mut weekly {
        occurrence.series_key = Some("weekly".into());
    }
    let [first, second] = weekly;
    let SyncedCalendar {
        service,
        repo,
        settings,
    } = synced_calendar(vec![first, second, meeting("one-off", 3)]).await;
    let clock: Arc<dyn Clock> = Arc::new(clock());
    let rules = AutoRecordService::new(AutoRecordDeps {
        calendar: service,
        events: repo.clone(),
        series: repo,
        settings: settings.clone(),
        clock,
    });
    Harness { rules, settings }
}

/// Each planned event's provider ID with whether it records and why.
async fn plan(rules: &AutoRecordService) -> Vec<(String, bool, RecordReason)> {
    let range = TimeRange {
        start: hours(0),
        end: hours(48),
    };
    let planned = rules.plan(range).await.unwrap();
    planned
        .into_iter()
        .map(|it| {
            (
                it.event.provider_event_id,
                it.decision.record,
                it.decision.reason,
            )
        })
        .collect()
}

async fn id_of(rules: &AutoRecordService, provider_id: &str) -> CalendarEventId {
    let range = TimeRange {
        start: hours(0),
        end: hours(48),
    };
    let planned = rules.plan(range).await.unwrap();
    planned
        .into_iter()
        .find(|it| it.event.provider_event_id == provider_id)
        .map(|it| it.event.id)
        .unwrap()
}

fn choice(record: Option<bool>, scope: ChoiceScope) -> RecordChoice {
    RecordChoice { record, scope }
}

fn row(id: &str, record: bool, reason: RecordReason) -> (String, bool, RecordReason) {
    (id.to_owned(), record, reason)
}

#[tokio::test]
async fn the_plan_follows_the_switches() {
    let Harness { rules, settings } = harness().await;
    let off = plan(&rules).await;
    assert!(
        off.iter()
            .all(|(_, record, reason)| !record && *reason == RecordReason::AudienceOff)
    );

    settings
        .update(|settings| settings.calendar.auto_record.external = true)
        .await
        .unwrap();
    let on = plan(&rules).await;
    assert!(
        on.iter()
            .all(|(_, record, reason)| *record && *reason == RecordReason::Rules)
    );
}

#[tokio::test]
async fn just_this_one_leaves_the_other_occurrences_alone() {
    let Harness { rules, .. } = harness().await;
    let first = id_of(&rules, "weekly-1").await;

    let decision = rules
        .choose(first, choice(Some(true), ChoiceScope::ThisEvent))
        .await
        .unwrap();
    assert!(decision.record);
    assert_eq!(
        plan(&rules).await,
        [
            row("weekly-1", true, RecordReason::EventChoice),
            row("one-off", false, RecordReason::AudienceOff),
            row("weekly-2", false, RecordReason::AudienceOff),
        ]
    );
}

#[tokio::test]
async fn all_occurrences_overrides_earlier_single_choices_and_clears() {
    let Harness { rules, .. } = harness().await;
    let first = id_of(&rules, "weekly-1").await;
    let second = id_of(&rules, "weekly-2").await;
    rules
        .choose(second, choice(Some(true), ChoiceScope::ThisEvent))
        .await
        .unwrap();

    rules
        .choose(first, choice(Some(false), ChoiceScope::AllOccurrences))
        .await
        .unwrap();
    let series = plan(&rules).await;
    assert_eq!(
        series[0],
        row("weekly-1", false, RecordReason::SeriesChoice)
    );
    assert_eq!(
        series[2],
        row("weekly-2", false, RecordReason::SeriesChoice)
    );

    rules
        .choose(second, choice(None, ChoiceScope::AllOccurrences))
        .await
        .unwrap();
    let cleared = plan(&rules).await;
    assert!(
        cleared
            .iter()
            .all(|(_, _, reason)| *reason == RecordReason::AudienceOff)
    );
}

#[tokio::test]
async fn all_occurrences_of_a_one_off_is_just_this_one() {
    let Harness { rules, .. } = harness().await;
    let one_off = id_of(&rules, "one-off").await;
    let decision = rules
        .choose(one_off, choice(Some(true), ChoiceScope::AllOccurrences))
        .await
        .unwrap();
    assert_eq!(decision.reason, RecordReason::EventChoice);
    assert!(
        !plan(&rules)
            .await
            .iter()
            .any(|(_, _, r)| *r == RecordReason::SeriesChoice)
    );
}
