//! [`GenerateService`] over the seeded fixture meetings and a scripted
//! model: what runs, what is stored, and how failures are kept.

use std::collections::BTreeMap;
use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{
    Condition, ConditionGroups, Email, FakeClock, FixedZone, InsightStatus, MeetingId, Millis,
};
use deino_llm::{LlmError, ProviderId};
use deino_testkit::{at, seed, seed_fixtures};

use super::*;
use crate::enhance::MaterialSources;
use crate::insights::{
    ApplyWhen, Cardinality, FieldKind, FieldValue, FixedDomains, InsightField, InsightInput,
    InsightRun, InsightValue, ScriptedInsightModel, SqliteInsightRepo, WireAnswer, WireEvidence,
    WireInstance, WireScalar,
};
use crate::profile::InternalDomains;

const SALES: &str = "meeting-fixture-sales-discovery";

struct World {
    database: Database,
    repo: Arc<SqliteInsightRepo>,
    model: Arc<ScriptedInsightModel>,
    generator: GenerateService,
}

async fn world(model: ScriptedInsightModel) -> World {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let repo = Arc::new(SqliteInsightRepo::new(database.clone()));
    let model = Arc::new(model);
    let me = Email::parse("shreyas@deino.dev").unwrap();
    let generator = GenerateService::new(GenerateDeps {
        defs: repo.clone(),
        results: repo.clone(),
        records: repo.clone(),
        domains: Arc::new(FixedDomains(InternalDomains::from_emails(&[me]))),
        material: MaterialSources::sqlite(&database),
        model: model.clone(),
        clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
        zone: Arc::new(FixedZone::hours(-7).unwrap()),
    });
    World {
        database,
        repo,
        model,
        generator,
    }
}

/// Creates `input` straight through the repository.
async fn define(world: &World, input: InsightInput) -> InsightDef {
    let clock = FakeClock::new(at("2026-10-09T08:00:00Z"));
    let def = InsightDef {
        id: InsightId::generate(&clock),
        name: input.name,
        description: input.description,
        fields: input.fields,
        cardinality: input.cardinality,
        apply_when: input.apply_when,
        revision: 1,
        created_at: clock.now(),
        updated_at: clock.now(),
    };
    world.repo.create(&def).await.unwrap();
    def
}

fn input(name: &str, fields: Vec<InsightField>, apply_when: ApplyWhen) -> InsightInput {
    InsightInput {
        name: name.into(),
        description: format!("Find the {name}."),
        fields,
        cardinality: Cardinality::Auto,
        apply_when,
    }
}

fn external() -> ApplyWhen {
    ApplyWhen::Automatic {
        conditions: ConditionGroups(vec![vec![Condition::External { external: true }]]),
    }
}

fn budget_field() -> Vec<InsightField> {
    vec![InsightField {
        name: "Budget".into(),
        kind: FieldKind::Number,
    }]
}

fn budget_answer(raw: &str, evidence: &str) -> WireAnswer {
    WireAnswer {
        answers: vec![WireInstance {
            value: None,
            fields: BTreeMap::from([("Budget".to_owned(), Some(WireScalar::Text(raw.into())))]),
            evidence: Some(WireEvidence::One(evidence.into())),
        }],
    }
}

async fn runs(world: &World) -> Vec<InsightRun> {
    world
        .repo
        .for_meeting(&MeetingId::new(SALES))
        .await
        .unwrap()
}

#[tokio::test]
async fn matching_automatic_insights_run_and_store_typed_attributed_answers() {
    let model = ScriptedInsightModel::default().answer("Budget", Ok(budget_answer("$40k", "s1")));
    let world = world(model).await;
    let budget = define(&world, input("Budget", budget_field(), external())).await;
    define(&world, input("Notes", Vec::new(), ApplyWhen::Manual)).await;
    let interview = ApplyWhen::Automatic {
        conditions: ConditionGroups(vec![vec![Condition::TitleContains {
            text: "interview".into(),
        }]]),
    };
    define(&world, input("Debrief", Vec::new(), interview)).await;

    let outcome = world
        .generator
        .generate(&MeetingId::new(SALES), Attempt::Retryable)
        .await;

    assert_eq!(outcome.unwrap(), GenerateOutcome::Answered { insights: 1 });
    let asked = world.model.asked();
    assert_eq!(asked.len(), 1);
    assert!(
        asked[0]
            .transcript
            .starts_with("[s1 00:00] Lena Okafor: Hi Marcus"),
        "{}",
        asked[0].transcript
    );
    assert!(asked[0].meeting.contains("Brightkite"));
    let [run] = runs(&world).await.try_into().unwrap();
    assert_eq!(
        (run.insight_id, run.status, run.revision),
        (budget.id, InsightStatus::Done, Some(1))
    );
    let answer = &run.answers[0];
    assert_eq!(
        answer.value.field("Budget"),
        Some(&FieldValue::Number(40_000.0))
    );
    assert_eq!(answer.speaker.as_deref(), Some("Lena Okafor"));
    assert_eq!(answer.timestamp_ms, Some(Millis(994)));
    assert!(answer.segment_id.is_some());
}

#[tokio::test]
async fn requested_runs_are_all_that_runs_when_there_are_any() {
    let model = ScriptedInsightModel::default().answer(
        "Notes",
        Ok(WireAnswer {
            answers: vec![WireInstance {
                value: Some(WireScalar::Text("A pilot in one region.".into())),
                ..WireInstance::default()
            }],
        }),
    );
    let world = world(model).await;
    define(&world, input("Budget", budget_field(), external())).await;
    let notes = define(&world, input("Notes", Vec::new(), ApplyWhen::Manual)).await;
    let meeting = MeetingId::new(SALES);
    world
        .repo
        .request(
            &notes.id,
            std::slice::from_ref(&meeting),
            at("2026-10-09T08:30:00Z"),
        )
        .await
        .unwrap();

    world
        .generator
        .generate(&meeting, Attempt::Retryable)
        .await
        .unwrap();

    let [run] = runs(&world).await.try_into().unwrap();
    assert_eq!(run.insight_id, notes.id);
    assert_eq!(
        run.answers[0].value,
        InsightValue::Text {
            text: "A pilot in one region.".into()
        }
    );
    assert_eq!(run.answers[0].speaker, None);
}

fn rate_limited() -> LlmError {
    LlmError::RateLimited {
        provider: ProviderId::Anthropic,
        retry_after: None,
    }
}

#[tokio::test]
async fn a_rate_limit_leaves_the_run_pending_for_the_retry_until_the_last_attempt() {
    let model = ScriptedInsightModel::default()
        .answer("Budget", Err(rate_limited()))
        .answer("Budget", Err(rate_limited()));
    let world = world(model).await;
    define(&world, input("Budget", budget_field(), external())).await;
    let meeting = MeetingId::new(SALES);

    let retried = world
        .generator
        .generate(&meeting, Attempt::Retryable)
        .await
        .unwrap_err();
    let pending = runs(&world).await;
    let last = world
        .generator
        .generate(&meeting, Attempt::Last)
        .await
        .unwrap_err();
    let failed = runs(&world).await;

    assert!(retried.is_retryable() && last.is_retryable());
    assert_eq!(pending[0].status, InsightStatus::Pending);
    assert_eq!(failed[0].status, InsightStatus::Failed);
    assert!(failed[0].error.is_some());
}

#[tokio::test]
async fn a_missing_key_is_stored_on_every_run_and_the_others_still_run() {
    let model = ScriptedInsightModel::default()
        .answer("Budget", Err(LlmError::NoKey(ProviderId::Anthropic)))
        .answer("Decisions", Ok(WireAnswer::default()));
    let world = world(model).await;
    define(&world, input("Budget", budget_field(), external())).await;
    define(&world, input("Decisions", Vec::new(), external())).await;

    let error = world
        .generator
        .generate(&MeetingId::new(SALES), Attempt::Retryable)
        .await
        .unwrap_err();

    assert!(!error.is_retryable());
    let statuses: Vec<InsightStatus> = runs(&world).await.iter().map(|run| run.status).collect();
    assert!(statuses.contains(&InsightStatus::Failed) && statuses.contains(&InsightStatus::Done));
}

#[tokio::test]
async fn nothing_runs_when_no_insight_applies() {
    let world = world(ScriptedInsightModel::default()).await;
    define(&world, input("Notes", Vec::new(), ApplyWhen::Manual)).await;

    let outcome = world
        .generator
        .generate(&MeetingId::new(SALES), Attempt::Retryable)
        .await;

    assert_eq!(outcome.unwrap(), GenerateOutcome::NothingToRun);
    assert!(world.model.asked().is_empty());
}

#[tokio::test]
async fn an_insight_asked_of_a_meeting_without_transcript_fails_with_a_reason() {
    let world = world(ScriptedInsightModel::default()).await;
    seed::meeting(
        &world.database,
        "meeting-empty",
        "Empty",
        at("2026-10-09T07:00:00Z"),
    )
    .await;
    let notes = define(&world, input("Notes", Vec::new(), ApplyWhen::Manual)).await;
    let meeting = MeetingId::new("meeting-empty");
    world
        .repo
        .request(
            &notes.id,
            std::slice::from_ref(&meeting),
            at("2026-10-09T08:30:00Z"),
        )
        .await
        .unwrap();

    let outcome = world
        .generator
        .generate(&meeting, Attempt::Retryable)
        .await
        .unwrap();

    assert_eq!(outcome, GenerateOutcome::NoTranscript);
    let run = &world.repo.for_meeting(&meeting).await.unwrap()[0];
    assert_eq!(
        (run.status, run.error.as_deref()),
        (InsightStatus::Failed, Some(NO_TRANSCRIPT))
    );
}

#[tokio::test]
async fn the_answers_show_in_the_insights_table_at_the_cited_moment() {
    let model = ScriptedInsightModel::default().answer("Budget", Ok(budget_answer("12", "s2")));
    let world = world(model).await;
    let budget = define(&world, input("Budget", budget_field(), external())).await;
    world
        .generator
        .generate(&MeetingId::new(SALES), Attempt::Retryable)
        .await
        .unwrap();

    let table = world.repo.for_insight(&budget.id).await.unwrap();

    assert_eq!(table.len(), 1);
    assert_eq!(table[0].run.answers[0].timestamp_ms, Some(Millis(8_127)));
}
