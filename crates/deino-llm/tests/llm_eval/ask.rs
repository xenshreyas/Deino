//! `chat`, one meeting (AI-02-lite, D13): "Ask about this meeting" through
//! the whole `deino_core::ask` service over each seeded fixture meeting,
//! asking the first of its ground-truth facts.
//!
//! A good answer cites the transcript, every citation is a real segment of
//! the meeting (the service drops the rest), and one cites the fact's
//! evidence line or a line next to it.
//!
//! `contract::chat::<meeting>` checks the prompt carries the question and the
//! evidence line with its reference; `smoke::…` scores an oracle answer
//! (the expected answer citing the evidence line, plus an invented reference
//! the service must drop) through `MockProvider`; `live::…` asks a real model
//! (`DEINO_EVAL=1`).

use std::sync::Arc;

use deino_core::ask::{
    AskDeps, AskMessage, AskRequest, AskService, GatewayAskModel, SqliteAskRepo,
};
use deino_core::enhance::MaterialSources;
use deino_core::recipes::{RecipeService, SqliteRecipeRepo};
use deino_core::segments::{Page, Segment, SegmentReader, SqliteSegmentRepo};
use deino_db::test_db;
use deino_domain::SystemClock;
use deino_llm::{CompletionRequest, Gateway, MockProvider, Route, TextSink, mock_gateway};
use deino_testkit::fixture::{ExpectedFact, FixtureMeeting, Fixtures, seed_fixtures};
use libtest_mimic::{Failed, Trial};

/// The route under test.
const ROUTE: Route = Route::Chat;

/// One question about one meeting.
#[derive(Clone)]
struct AskCase {
    meeting: FixtureMeeting,
    fact: ExpectedFact,
}

/// What a run produced: the answer, the meeting's segments, the requests.
struct Run {
    answer: AskMessage,
    segments: Vec<Segment>,
    requests: Vec<CompletionRequest>,
}

/// Every ask trial.
pub fn trials(live_enabled: bool) -> Vec<Trial> {
    let Ok(fixtures) = Fixtures::load() else {
        return vec![Trial::test("setup::chat", || {
            Err("could not load the fixtures".into())
        })];
    };
    fixtures
        .meetings()
        .iter()
        .filter_map(|meeting| {
            let fact = meeting.ground_truth.facts.first()?.clone();
            Some(AskCase {
                meeting: meeting.clone(),
                fact,
            })
        })
        .flat_map(|case| case_trials(case, live_enabled))
        .collect()
}

fn case_trials(case: AskCase, live_enabled: bool) -> Vec<Trial> {
    let name = case.meeting.id.to_string().replace("meeting-fixture-", "");
    let (contract, smoke, live) = (case.clone(), case.clone(), case);
    vec![
        Trial::test(format!("contract::{ROUTE}::{name}"), move || {
            check_prompt(&contract)
        }),
        Trial::test(format!("smoke::{ROUTE}::{name}"), move || {
            let run = ask(&smoke, oracle(&smoke.fact))?;
            judge(&smoke, &run).map_err(Failed::from)
        }),
        Trial::test(format!("live::{ROUTE}::{name}"), move || {
            let run = ask(&live, crate::live::gateway_for(ROUTE)?)?;
            judge(&live, &run).map_err(Failed::from)
        })
        .with_ignored_flag(!live_enabled),
    ]
}

/// The `s<n>` of the line in the request's last message holding `needle`.
fn reference_of(request: &CompletionRequest, needle: &str) -> Option<String> {
    let last = request.messages.last()?.text();
    let line = last.lines().find(|line| line.contains(needle))?;
    Some(line.trim_start_matches('[').split(' ').next()?.to_owned())
}

/// A gateway whose model answers with the fact, citing its evidence line
/// and a line that does not exist.
fn oracle(fact: &ExpectedFact) -> Gateway {
    let (answer, evidence) = (fact.answer.clone(), fact.evidence.clone());
    let mock = MockProvider::new().answer_with(ROUTE, move |request| {
        let reference = reference_of(request, &evidence).unwrap_or_default();
        format!("{answer} [{reference}] [s99999]")
    });
    mock_gateway(Arc::new(mock)).gateway
}

fn check_prompt(case: &AskCase) -> Result<(), Failed> {
    let mock = Arc::new(MockProvider::new().reply(ROUTE, "ok"));
    let run = ask_with(case, mock_gateway(mock.clone()).gateway, &mock)?;
    let request = run.requests.last().ok_or("no request was sent")?;
    let last = request
        .messages
        .last()
        .map(deino_llm::Message::text)
        .unwrap_or_default();
    if !last.ends_with(case.fact.question.as_str()) {
        return Err("the prompt does not end with the question".into());
    }
    match reference_of(request, &case.fact.evidence) {
        Some(reference) if reference.starts_with('s') => Ok(()),
        _ => Err("the prompt does not show the evidence line with its reference".into()),
    }
}

/// Whether the answer cites the evidence line (or a neighbour) and only
/// real segments.
fn judge(case: &AskCase, run: &Run) -> Result<(), String> {
    let position = |id| run.segments.iter().position(|s| s.id == id);
    let evidence = run
        .segments
        .iter()
        .position(|s| s.text.contains(&case.fact.evidence))
        .ok_or("the evidence line is not in the transcript")?;
    let cited: Vec<Option<usize>> = run
        .answer
        .parts
        .iter()
        .flat_map(|part| part.citations.iter().map(|c| position(c.segment_id)))
        .collect();
    if cited.is_empty() {
        return Err("the answer cites nothing".to_owned());
    }
    if cited.iter().any(Option::is_none) {
        return Err("the answer cites a segment of another meeting".to_owned());
    }
    cited
        .iter()
        .flatten()
        .any(|at| at.abs_diff(evidence) <= 1)
        .then_some(())
        .ok_or_else(|| format!("no citation near the evidence line ({evidence}): {cited:?}"))
}

fn ask(case: &AskCase, gateway: Gateway) -> Result<Run, Failed> {
    ask_with(case, gateway, &Arc::new(MockProvider::new()))
}

/// Seeds the fixtures and asks the case's question through the service.
fn ask_with(case: &AskCase, gateway: Gateway, mock: &Arc<MockProvider>) -> Result<Run, Failed> {
    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    runtime.block_on(async {
        let database = test_db().await;
        seed_fixtures(&database)
            .await
            .map_err(|error| error.to_string())?;
        let clock = Arc::new(SystemClock);
        let service = AskService::new(AskDeps {
            sources: MaterialSources::sqlite(&database),
            store: Arc::new(SqliteAskRepo::new(database.clone())),
            recipes: Arc::new(RecipeService::new(
                Arc::new(SqliteRecipeRepo::new(database.clone())),
                clock.clone(),
            )),
            model: Arc::new(GatewayAskModel::new(Arc::new(gateway))),
            clock,
        });
        let request = AskRequest {
            question: case.fact.question.clone(),
            recipe: None,
        };
        let answer = service
            .ask(&case.meeting.id, request, &Quiet)
            .await
            .map_err(|error| Failed::from(format!("ask failed: {error:?}")))?;
        let segments = SqliteSegmentRepo::new(database)
            .list(&case.meeting.id, Page::ALL)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Run {
            answer,
            segments,
            requests: mock.requests(),
        })
    })
}

/// Drops the streamed pieces.
struct Quiet;

impl TextSink for Quiet {
    fn text(&self, _piece: &str) {}
}
