//! [`InsightService`] over in-memory ports: definitions, the views, runs on
//! past meetings and the CSV.

use std::path::PathBuf;
use std::sync::Arc;

use deino_domain::{
    Condition, ConditionGroups, Email, ErrorClass, ErrorCode, FakeClock, FixedZone, InsightStatus,
    MeetingFacts, MeetingId, Millis, SegmentId,
};
use deino_testkit::at;

use super::*;
use crate::export::{FakeVault, ScriptedDialog};
use crate::insights::{
    ApplyWhen, Cardinality, FakeInsightStore, FieldEntry, FieldKind, FieldValue, FixedDomains,
    InsightAnswer, InsightField, InsightResultRepo, InsightValue, MeetingRecord, PastMeeting,
    RecordedScheduler, RunAnswers, gallery,
};
use crate::profile::InternalDomains;

const ACME: &str = "meeting-acme";
const STANDUP: &str = "meeting-standup";

struct World {
    service: InsightService,
    store: Arc<FakeInsightStore>,
    scheduler: Arc<RecordedScheduler>,
    vault: Arc<FakeVault>,
}

fn world_with(dialog: Option<PathBuf>) -> World {
    let store = Arc::new(FakeInsightStore::default());
    meeting(
        &store,
        ACME,
        "Acme discovery",
        &["maya@acme.com", "me@deino.dev"],
    );
    meeting(
        &store,
        STANDUP,
        "Eng standup",
        &["owen@deino.dev", "me@deino.dev"],
    );
    let scheduler = Arc::new(RecordedScheduler::default());
    let vault = Arc::new(FakeVault::default());
    let me = Email::parse("me@deino.dev").unwrap();
    let ports = InsightPorts {
        defs: store.clone(),
        results: store.clone(),
        records: store.clone(),
        domains: Arc::new(FixedDomains(InternalDomains::from_emails(&[me]))),
        names: store.clone(),
        scheduler: scheduler.clone(),
        files: InsightFiles {
            save_dialog: Arc::new(ScriptedDialog::answering(dialog)),
            vault: vault.clone(),
            zone: Arc::new(FixedZone::hours(-7).unwrap()),
        },
        clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
    };
    World {
        service: InsightService::new(ports),
        store,
        scheduler,
        vault,
    }
}

fn world() -> World {
    world_with(None)
}

fn meeting(store: &FakeInsightStore, id: &str, title: &str, emails: &[&str]) {
    let attendee_emails: Vec<Email> = emails
        .iter()
        .map(|email| Email::parse(email).unwrap())
        .collect();
    let past = PastMeeting {
        id: MeetingId::new(id),
        title: title.into(),
        started_at: if id == ACME {
            at("2026-10-08T16:00:00Z")
        } else {
            at("2026-10-07T16:00:00Z")
        },
    };
    let facts = MeetingFacts {
        title: title.into(),
        attendee_domains: attendee_emails
            .iter()
            .map(|email| email.domain().to_owned())
            .collect(),
        ..MeetingFacts::default()
    };
    store.add_meeting(
        past,
        MeetingRecord {
            facts,
            attendee_emails,
        },
    );
}

fn template(key: &str) -> InsightInput {
    gallery().into_iter().find(|t| t.key == key).unwrap().input
}

#[tokio::test]
async fn creating_from_a_template_stores_revision_one_and_refuses_a_taken_name() {
    let world = world();

    let created = world
        .service
        .create(template("discovery_scorecard"))
        .await
        .unwrap();
    let again = world
        .service
        .create(template("discovery_scorecard"))
        .await
        .unwrap_err();

    assert_eq!(
        (created.name.as_str(), created.revision),
        ("Discovery scorecard", 1)
    );
    assert_eq!(world.service.get(&created.id).await.unwrap(), created);
    assert_eq!(again.code(), ErrorCode::Conflict);
}

#[tokio::test]
async fn editing_what_an_insight_asks_bumps_its_revision_but_editing_when_it_runs_does_not() {
    let world = world();
    let created = world.service.create(template("risks")).await.unwrap();

    let manual = InsightInput {
        apply_when: ApplyWhen::Manual,
        ..template("risks")
    };
    let same_question = world.service.update(&created.id, manual).await.unwrap();
    let reworded = InsightInput {
        description: "List the risks, worst first.".into(),
        ..template("risks")
    };
    let new_question = world.service.update(&created.id, reworded).await.unwrap();

    assert_eq!(same_question.revision, 1);
    assert_eq!(same_question.apply_when, ApplyWhen::Manual);
    assert_eq!(new_question.revision, 2);
}

#[tokio::test]
async fn missing_insights_are_not_found_and_invalid_input_is_refused() {
    let world = world();
    let missing = deino_domain::InsightId::generate(&FakeClock::new(at("2026-10-09T09:00:00Z")));
    let blank = InsightInput {
        name: " ".into(),
        ..template("risks")
    };

    assert_eq!(
        world.service.delete(&missing).await.unwrap_err().code(),
        ErrorCode::NotFound
    );
    assert_eq!(
        world.service.create(blank).await.unwrap_err().code(),
        ErrorCode::Invalid
    );
}

#[tokio::test]
async fn running_on_past_meetings_queues_each_once_and_schedules_its_stage() {
    let world = world();
    let insight = world.service.create(template("decisions")).await.unwrap();
    let meetings = [
        MeetingId::new(ACME),
        MeetingId::new(STANDUP),
        MeetingId::new(ACME),
    ];

    let progress = world.service.run(&insight.id, &meetings).await.unwrap();

    assert_eq!(
        progress,
        RunProgress {
            pending: 2,
            done: 0,
            failed: 0
        }
    );
    assert_eq!(world.scheduler.scheduled(), meetings[..2]);
}

#[tokio::test]
async fn a_run_fails_only_when_no_meeting_could_be_scheduled() {
    let world = world();
    let insight = world.service.create(template("decisions")).await.unwrap();
    world.scheduler.refuse(MeetingId::new(ACME));

    let partly = world
        .service
        .run(
            &insight.id,
            &[MeetingId::new(ACME), MeetingId::new(STANDUP)],
        )
        .await;
    let wholly = world
        .service
        .run(&insight.id, &[MeetingId::new(ACME)])
        .await
        .unwrap_err();

    assert!(partly.is_ok());
    assert_eq!(wholly.code(), ErrorCode::Unavailable);
    assert_eq!(
        wholly.to_string(),
        "could not schedule the run: the meeting is still recording"
    );
    let unknown = world
        .service
        .run(&insight.id, &[MeetingId::new("nope")])
        .await
        .unwrap_err();
    assert_eq!(unknown.code(), ErrorCode::NotFound);
    assert_eq!(
        world
            .service
            .run(&insight.id, &[])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::Invalid
    );
}

fn answer(segment: SegmentId, budget: f64) -> InsightAnswer {
    InsightAnswer {
        value: InsightValue::Fields {
            fields: vec![FieldEntry {
                name: "Budget".into(),
                value: Some(FieldValue::Number(budget)),
            }],
        },
        speaker: Some("Speaker 2".into()),
        timestamp_ms: Some(Millis(271_000)),
        segment_id: Some(segment),
    }
}

async fn answered(world: &World, insight: &InsightDef, meeting: &str, budget: f64) -> SegmentId {
    let segment: SegmentId = "01920000-0000-7000-8000-0000000000aa".parse().unwrap();
    world
        .store
        .save_answers(&RunAnswers {
            insight_id: insight.id,
            meeting_id: MeetingId::new(meeting),
            revision: 1,
            answers: vec![answer(segment, budget)],
            at: at("2026-10-09T09:00:00Z"),
        })
        .await
        .unwrap();
    segment
}

fn budget_input(name: &str) -> InsightInput {
    InsightInput {
        name: name.into(),
        description: "The budget.".into(),
        fields: vec![InsightField {
            name: "Budget".into(),
            kind: FieldKind::Number,
        }],
        cardinality: Cardinality::Fixed { count: 1 },
        apply_when: ApplyWhen::Automatic {
            conditions: ConditionGroups(vec![vec![Condition::AttendeeDomain {
                domain: "acme.com".into(),
            }]]),
        },
    }
}

#[tokio::test]
async fn a_meetings_tab_lists_its_runs_with_current_speaker_names_and_offers_the_rest() {
    let world = world();
    let budget = world.service.create(budget_input("Budget")).await.unwrap();
    let other = world.service.create(template("decisions")).await.unwrap();
    let segment = answered(&world, &budget, ACME, 40_000.0).await;
    world.store.name_speaker(segment, "Maya Chen");
    let reworded = InsightInput {
        description: "The budget in dollars.".into(),
        ..budget_input("Budget")
    };
    world.service.update(&budget.id, reworded).await.unwrap();

    let tab = world
        .service
        .for_meeting(&MeetingId::new(ACME))
        .await
        .unwrap();

    let [entry] = tab.entries.try_into().unwrap();
    assert_eq!(entry.def.id, budget.id);
    assert!(entry.stale);
    assert_eq!(entry.run.answers[0].speaker.as_deref(), Some("Maya Chen"));
    assert_eq!(tab.others, [world.service.get(&other.id).await.unwrap()]);
    let missing = world
        .service
        .for_meeting(&MeetingId::new("nope"))
        .await
        .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
}

#[tokio::test]
async fn candidates_preselect_matching_meetings_and_show_their_status() {
    let world = world();
    let budget = world.service.create(budget_input("Budget")).await.unwrap();
    answered(&world, &budget, STANDUP, 1.0).await;

    let candidates = world.service.candidates(&budget.id).await.unwrap();

    let summary: Vec<(&str, bool, Option<InsightStatus>)> = candidates
        .iter()
        .map(|c| (c.meeting.id.as_str(), c.matches, c.status))
        .collect();
    assert_eq!(
        summary,
        [
            (ACME, true, None),
            (STANDUP, false, Some(InsightStatus::Done))
        ]
    );
}

#[tokio::test]
async fn the_table_exports_as_csv_where_the_user_saves_it() {
    let world = world_with(Some(PathBuf::from("/exports/budget")));
    let budget = world.service.create(budget_input("Budget")).await.unwrap();
    answered(&world, &budget, ACME, 40_000.0).await;

    let saved = world.service.export_csv(&budget.id).await.unwrap().unwrap();

    assert_eq!(saved.file_name, "budget.csv");
    let csv = world.vault.read("/exports/budget.csv").unwrap();
    assert_eq!(
        csv,
        "Meeting,Date,Budget,Speaker,Time,Link\r\n\
         Acme discovery,2026-10-08T09:00:00,40000,Speaker 2,04:31,deino://meetings/meeting-acme\r\n"
    );
}

#[tokio::test]
async fn a_cancelled_save_writes_nothing() {
    let world = world();
    let budget = world.service.create(budget_input("Budget")).await.unwrap();

    assert_eq!(world.service.export_csv(&budget.id).await.unwrap(), None);
    assert!(world.vault.paths().is_empty());
}

#[tokio::test]
async fn the_list_carries_each_insights_progress() {
    let world = world();
    let budget = world.service.create(budget_input("Budget")).await.unwrap();
    answered(&world, &budget, ACME, 1.0).await;

    let list = world.service.list().await.unwrap();

    assert_eq!(
        list[0].progress,
        RunProgress {
            pending: 0,
            done: 1,
            failed: 0
        }
    );
    assert_eq!(world.service.templates().len(), 12);
}
