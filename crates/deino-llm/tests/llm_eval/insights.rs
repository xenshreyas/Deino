//! `extract.insights` (AI-06): the whole insights engine over the seeded
//! fixture meetings, scored on what the user sees.
//!
//! - **Decisions** (the gallery template, free text, Auto) on every fixture:
//!   recall of the ground truth's decisions, and the share of answers
//!   attributed to a real moment (speaker and time).
//! - **Discovery scorecard** (typed fields) on the sales call: one answer,
//!   every field of its type, the score between 1 and 5.
//!
//! Trials:
//!
//! - `contract::extract.insights::<fixture>`: the prompt carries the
//!   insight, its count and the numbered transcript, and the schema asks for
//!   `answers`;
//! - `smoke::extract.insights::<fixture>`: the ground truth's decisions as
//!   the answer, through `MockProvider` and the engine, score 1.0;
//! - `smoke::extract.insights::typed-fields-repaired`: a loose scorecard
//!   (`"$40k"`, `"4"`) is stored typed;
//! - `smoke::extract.insights::scoring-rejects-nothing`: no answers fail;
//! - `live::extract.insights::decisions` and `…::discovery-scorecard`: a
//!   real model (`DEINO_EVAL=1`); decisions pass at recall ≥ 0.6 over all
//!   fixtures with ≥ 0.9 of answers attributed.

mod engine;
mod score;

use std::sync::Arc;

use deino_core::insights::FieldValue;
use deino_llm::{MockProvider, OutputFormat, Route, mock_gateway};
use deino_testkit::fixture::{FixtureMeeting, Fixtures, MeetingKind};
use libtest_mimic::{Failed, Trial};

use self::engine::{decisions_answer, run_template};
use self::score::{Tally, check_decisions, check_scorecard};
use crate::live;

/// The route under test, as the trial names spell it.
const ROUTE: Route = Route::ExtractInsights;

/// Every insights trial.
pub fn trials(live_enabled: bool) -> Vec<Trial> {
    let Ok(fixtures) = Fixtures::load() else {
        return vec![Trial::test("setup::extract.insights", || {
            Err("could not load the fixtures".into())
        })];
    };
    let meetings = fixtures.meetings().to_vec();
    let mut trials: Vec<Trial> = meetings.iter().flat_map(meeting_trials).collect();
    let sales = fixtures.meeting(MeetingKind::SalesDiscovery).cloned();
    trials.extend(shared_trials(meetings, sales, live_enabled));
    trials
}

fn meeting_trials(meeting: &FixtureMeeting) -> Vec<Trial> {
    let slug = meeting.id.as_str().trim_start_matches("meeting-fixture-");
    let (contract, smoke) = (meeting.clone(), meeting.clone());
    vec![
        Trial::test(format!("contract::{ROUTE}::{slug}"), move || {
            run_contract(&contract)
        }),
        Trial::test(format!("smoke::{ROUTE}::{slug}"), move || run_smoke(&smoke)),
    ]
}

fn shared_trials(
    meetings: Vec<FixtureMeeting>,
    sales: Option<FixtureMeeting>,
    live_enabled: bool,
) -> Vec<Trial> {
    let (typed, live_sales) = (sales.clone(), sales);
    let empty = meetings.first().cloned();
    vec![
        Trial::test(
            format!("smoke::{ROUTE}::typed-fields-repaired"),
            move || run_typed_smoke(typed.as_ref()),
        ),
        Trial::test(
            format!("smoke::{ROUTE}::scoring-rejects-nothing"),
            move || match empty.map(|meeting| check_decisions(&meeting, &[])) {
                Some(Err(_)) => Ok(()),
                _ => Err("no answers passed the scoring".into()),
            },
        ),
        Trial::test(format!("live::{ROUTE}::decisions"), move || {
            run_live(&meetings)
        })
        .with_ignored_flag(!live_enabled),
        Trial::test(format!("live::{ROUTE}::discovery-scorecard"), move || {
            run_live_scorecard(live_sales.as_ref())
        })
        .with_ignored_flag(!live_enabled),
    ]
}

/// The prompt names the insight and carries the numbered transcript.
fn run_contract(meeting: &FixtureMeeting) -> Result<(), Failed> {
    let mock = Arc::new(MockProvider::new().reply(ROUTE, &decisions_answer(meeting)));
    run_template(meeting, "decisions", mock_gateway(mock.clone()).gateway)?;
    let request = mock
        .requests()
        .into_iter()
        .next()
        .ok_or("the model was never asked")?;
    let user: String = request
        .messages
        .iter()
        .map(deino_llm::Message::text)
        .collect();
    let needed = [
        "Name: Decisions",
        "Answers: as many as the meeting holds",
        "[s1 00:",
    ];
    if let Some(missing) = needed.iter().find(|fragment| !user.contains(**fragment)) {
        return Err(format!("the prompt lacks {missing:?}").into());
    }
    match &request.output {
        OutputFormat::Json(json) if json.schema["required"] == serde_json::json!(["answers"]) => {
            Ok(())
        }
        _ => Err("the request does not ask for the answers schema".into()),
    }
}

/// The ground truth through the engine scores perfectly.
fn run_smoke(meeting: &FixtureMeeting) -> Result<(), Failed> {
    let mock = Arc::new(MockProvider::new().reply(ROUTE, &decisions_answer(meeting)));
    let answers = run_template(meeting, "decisions", mock_gateway(mock).gateway)?;
    check_decisions(meeting, &answers)
        .map(|_| ())
        .map_err(Failed::from)
}

/// A loose scorecard is repaired into typed values.
fn run_typed_smoke(sales: Option<&FixtureMeeting>) -> Result<(), Failed> {
    let sales = sales.ok_or("no sales fixture")?;
    let loose = r#"{"answers":[{"fields":{"Questions":"Is SSO included?","Pain":"Manual dispatch",
        "Budget":"$40k","Timeline":"Q1 pilot","Decision-maker":"Marcus Webb","Score":"4"},"evidence":"s3"}]}"#;
    let mock = Arc::new(MockProvider::new().reply(ROUTE, loose));
    let answers = run_template(sales, "discovery_scorecard", mock_gateway(mock).gateway)?;
    check_scorecard(&answers)?;
    match answers
        .first()
        .and_then(|answer| answer.value.field("Budget"))
    {
        Some(FieldValue::Number(budget)) if (*budget - 40_000.0).abs() < f64::EPSILON => Ok(()),
        other => Err(format!("the budget was not repaired: {other:?}").into()),
    }
}

/// A real model's decisions, over every fixture.
fn run_live(meetings: &[FixtureMeeting]) -> Result<(), Failed> {
    let mut tally = Tally::default();
    for meeting in meetings {
        let answers = run_template(meeting, "decisions", live::gateway_for(ROUTE)?)?;
        tally += score::tally_decisions(meeting, &answers);
    }
    eprintln!("extract.insights decisions: {}", tally.summary());
    tally.passes().map_err(Failed::from)
}

/// A real model's scorecard of the sales call.
fn run_live_scorecard(sales: Option<&FixtureMeeting>) -> Result<(), Failed> {
    let sales = sales.ok_or("no sales fixture")?;
    let answers = run_template(sales, "discovery_scorecard", live::gateway_for(ROUTE)?)?;
    check_scorecard(&answers).map_err(Failed::from)
}
