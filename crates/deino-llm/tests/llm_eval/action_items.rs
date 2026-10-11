//! `extract.action_items` (AI-05): the action item prompt over every fixture
//! meeting, scored against its `groundTruth` (`scoring.rs`).
//!
//! - `contract::extract.action_items::<fixture>`: the prompt carries the
//!   instructions, the people (the user marked), the notes and the numbered
//!   transcript, and every ground-truth evidence quote is in it;
//! - `smoke::extract.action_items::<fixture>`: the ground truth itself,
//!   answered through `MockProvider` and `Gateway::structured`, scores 1.0;
//! - `smoke::extract.action_items::distractors_lower_precision`: the scorer
//!   counts a floated idea as an extra item;
//! - `live::extract.action_items::fixtures`: a real provider over all eight
//!   fixtures; passes at precision ≥ 0.8 (the card's bar) and recall ≥ 0.5.
//!
//! The prompt is built the way `deino_core::action_items::extraction`
//! builds it, with the same `deino_llm::extraction` line helpers, in the
//! fixtures' time zone (US Pacific, UTC-7 in September and October 2026).

use std::sync::Arc;

use chrono::FixedOffset;
use deino_llm::extraction::{
    ActionItemsAnswer, ExtractedActionItem, ME_SPEAKER, attendee_line, meeting_header,
    transcript_line,
};
use deino_llm::prompts::{ActionItemsSystem, ActionItemsUser, PromptText};
use deino_llm::{Gateway, MockProvider, Prompt, Route, mock_gateway};
use deino_testkit::fixture::{FixtureMeeting, Fixtures, USER_EMAIL};
use libtest_mimic::{Failed, Trial};

use crate::scoring::{Score, score};

/// The route under test, as the trial names spell it.
const ROUTE: Route = Route::ExtractActionItems;
/// The card's bar: at least this share of extracted items are real.
const MIN_PRECISION: f64 = 0.8;
/// A floor that keeps an empty answer from passing.
const MIN_RECALL: f64 = 0.5;
/// The fixtures' offset from UTC, in seconds west (Pacific daylight time).
const FIXTURE_SECONDS_WEST: i32 = 7 * 3_600;

/// Every action item trial.
pub fn trials(live_enabled: bool) -> Vec<Trial> {
    let fixtures = match Fixtures::load() {
        Ok(fixtures) => fixtures.meetings().to_vec(),
        Err(error) => {
            let message = format!("could not load the fixtures: {error}");
            return vec![Trial::test("setup::action_items", move || {
                Err(message.into())
            })];
        }
    };
    let mut trials = Vec::new();
    for meeting in &fixtures {
        let (contract, smoke) = (meeting.clone(), meeting.clone());
        trials.push(Trial::test(name("contract", meeting), move || {
            check_prompt(&contract)
        }));
        trials.push(Trial::test(name("smoke", meeting), move || {
            smoke_truth(&smoke)
        }));
    }
    let distractors = fixtures.clone();
    trials.push(Trial::test(
        format!("smoke::{ROUTE}::distractors_lower_precision"),
        move || smoke_distractors(&distractors),
    ));
    trials.push(
        Trial::test(format!("live::{ROUTE}::fixtures"), move || live(&fixtures))
            .with_ignored_flag(!live_enabled),
    );
    trials
}

fn name(kind: &str, meeting: &FixtureMeeting) -> String {
    let slug = meeting.id.as_str().trim_start_matches("meeting-fixture-");
    format!("{kind}::{ROUTE}::{slug}")
}

/// The extraction prompt for a fixture meeting.
pub fn prompt(meeting: &FixtureMeeting) -> Result<Prompt, Failed> {
    let offset = FixedOffset::west_opt(FIXTURE_SECONDS_WEST).ok_or("an invalid fixture offset")?;
    let attendees: Vec<String> = meeting
        .attendees
        .iter()
        .map(|person| {
            let is_me = person.email.as_str() == USER_EMAIL;
            attendee_line(&person.name, is_me, person.title.as_deref())
        })
        .collect();
    let lines: Vec<String> = meeting
        .segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let speaker = speaker_name(meeting, &segment.speaker);
            transcript_line(index, segment.start_ms.get(), &speaker, &segment.text)
        })
        .collect();
    let user = ActionItemsUser {
        meeting: &meeting_header(&meeting.title, meeting.started_at.with_timezone(&offset)),
        attendees: &attendees.join("\n"),
        notes: meeting.user_notes.as_deref(),
        transcript: &lines.join("\n"),
    };
    Ok(Prompt::new(user.text())
        .system(ActionItemsSystem.text())
        .max_tokens(4_096))
}

/// The name a speaker label goes by: the attendee's, "Me" for the user, or
/// the label for someone who was not invited.
fn speaker_name(meeting: &FixtureMeeting, label: &str) -> String {
    let attendee = meeting
        .speakers
        .iter()
        .find(|speaker| speaker.label == label)
        .and_then(|speaker| speaker.attendee.as_ref());
    match attendee {
        Some(email) if email.as_str() == USER_EMAIL => ME_SPEAKER.to_owned(),
        Some(email) => meeting
            .attendees
            .iter()
            .find(|person| person.email == *email)
            .map_or_else(|| label.to_owned(), |person| person.name.clone()),
        None => label.to_owned(),
    }
}

fn check_prompt(meeting: &FixtureMeeting) -> Result<(), Failed> {
    let prompt = prompt(meeting)?;
    let user: String = prompt
        .messages
        .iter()
        .map(deino_llm::Message::text)
        .collect();
    let mut required = vec![
        "[s0 ".to_owned(),
        " (me)".to_owned(),
        format!("Title: {}", meeting.title),
    ];
    required.extend(
        meeting
            .ground_truth
            .action_items
            .iter()
            .map(|item| item.evidence.clone()),
    );
    if meeting.user_notes.is_some() {
        required.push("<user_notes>".to_owned());
    }
    let system = prompt.system.unwrap_or_default();
    if !system.contains("clear commitments") {
        return Err("the system prompt lacks the commitment rule".into());
    }
    match required
        .iter()
        .find(|fragment| !user.contains(fragment.as_str()))
    {
        Some(missing) => Err(format!("the prompt lacks {missing:?}").into()),
        None => Ok(()),
    }
}

/// The ground truth as the model would answer it.
fn truth_answer(meeting: &FixtureMeeting) -> ActionItemsAnswer {
    let items = meeting
        .ground_truth
        .action_items
        .iter()
        .map(|item| ExtractedActionItem {
            title: item.title.clone(),
            assignee: item
                .assignee
                .as_ref()
                .map(|email| attendee_name(meeting, email.as_str())),
            due: item.due.map(|due| due.to_string()),
            evidence_quote: item.evidence.clone(),
            ..ExtractedActionItem::default()
        })
        .collect();
    ActionItemsAnswer { items }
}

fn attendee_name(meeting: &FixtureMeeting, email: &str) -> String {
    if email == USER_EMAIL {
        return "me".to_owned();
    }
    meeting
        .attendees
        .iter()
        .find(|person| person.email.as_str() == email)
        .map_or_else(|| email.to_owned(), |person| person.name.clone())
}

/// Answers `answer` through the real gateway's structured output.
fn through_gateway(
    meeting: &FixtureMeeting,
    answer: &ActionItemsAnswer,
) -> Result<ActionItemsAnswer, Failed> {
    let json = serde_json::to_string(answer).map_err(|error| Failed::from(error.to_string()))?;
    let gateway = mock_gateway(Arc::new(MockProvider::new().reply(ROUTE, &json))).gateway;
    block_on(gateway.structured::<ActionItemsAnswer>(ROUTE, prompt(meeting)?))
}

fn smoke_truth(meeting: &FixtureMeeting) -> Result<(), Failed> {
    let answer = through_gateway(meeting, &truth_answer(meeting))?;
    let score = score(meeting, &answer.items);
    let truth = meeting.ground_truth.action_items.len();
    let perfect = score.matched == truth
        && score.extra == 0
        && score.right_assignee == truth
        && score.right_due == truth;
    if perfect {
        Ok(())
    } else {
        Err(format!("ground truth scored {}", score.summary()).into())
    }
}

fn smoke_distractors(meetings: &[FixtureMeeting]) -> Result<(), Failed> {
    let meeting = meetings
        .first()
        .ok_or_else(|| Failed::from("no fixtures"))?;
    let mut answer = truth_answer(meeting);
    answer.items.push(ExtractedActionItem {
        title: "Maybe look into a mobile redesign someday".into(),
        evidence_quote: "a floated idea nobody took on".into(),
        ..ExtractedActionItem::default()
    });
    let score = score(meeting, &through_gateway(meeting, &answer)?.items);
    if score.extra == 1 && score.precision() < 1.0 {
        Ok(())
    } else {
        Err(format!("a distractor scored {}", score.summary()).into())
    }
}

/// Every fixture, `DEINO_EVAL_SAMPLES` times, against a real provider.
fn live(meetings: &[FixtureMeeting]) -> Result<(), Failed> {
    let gateway = crate::live::gateway();
    let mut total = Score::default();
    for meeting in meetings {
        for _ in 0..crate::live::samples() {
            let answer = sample(&gateway, meeting)?;
            total += score(meeting, &answer.items);
        }
    }
    #[allow(
        clippy::print_stdout,
        reason = "the eval reports its scores to whoever ran it"
    )]
    {
        println!(
            "extract.action_items over {} fixtures: {}",
            meetings.len(),
            total.summary()
        );
    }
    if total.precision() >= MIN_PRECISION && total.recall() >= MIN_RECALL {
        Ok(())
    } else {
        let bar = format!("precision ≥ {MIN_PRECISION}, recall ≥ {MIN_RECALL}");
        Err(format!("below the bar ({bar}): {}", total.summary()).into())
    }
}

fn sample(gateway: &Gateway, meeting: &FixtureMeeting) -> Result<ActionItemsAnswer, Failed> {
    let mut prompt = prompt(meeting)?;
    prompt.target = crate::live::target();
    block_on(gateway.structured::<ActionItemsAnswer>(ROUTE, prompt))
}

fn block_on<T>(future: impl Future<Output = Result<T, deino_llm::LlmError>>) -> Result<T, Failed> {
    let runtime =
        tokio::runtime::Runtime::new().map_err(|error| Failed::from(error.to_string()))?;
    runtime
        .block_on(future)
        .map_err(|error| Failed::from(error.to_string()))
}
