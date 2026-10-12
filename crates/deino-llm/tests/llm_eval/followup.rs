//! `followup.email` (NOT-07): the decision and the draft over two fixture
//! meetings, rendered the way `deino_core::followups` renders them (people
//! marked `(me)`, `(external)`, the user's notes, `Speaker: text` lines).
//!
//! - `sales_discovery`: an external first call with agreed next steps. A good
//!   decision is `send`; a good draft has a subject of at most 80
//!   characters, greets Marcus or Hannah, mentions the pilot, and leaves at
//!   least one `[placeholder]` for what only the user knows.
//! - `colleague_owns`: the same call, where a colleague says she will send
//!   the recap herself. A good decision is anything but `send`.
//!
//! Trials: `contract::followup.email::<case>` (the prompts carry the rules
//! and the people), `smoke::followup.email::<case>` (known-good answers
//! through `MockProvider` and `Gateway::structured` pass the checks),
//! `live::followup.email::<case>` (a real provider; ignored unless
//! `DEINO_EVAL=1`).

use std::sync::Arc;

use deino_llm::followup::{FollowUpDecision, FollowUpDraftAnswer, FollowUpVerdict, placeholders};
use deino_llm::prompts::{
    FOLLOW_UP_DECIDE_TASK, FOLLOW_UP_DRAFT_TASK, FollowUpDecideSystem, FollowUpDraftSystem,
    FollowUpUser, PromptText,
};
use deino_llm::{Gateway, MockProvider, Prompt, Route, mock_gateway};
use deino_testkit::fixture::{FixtureMeeting, Fixtures, MeetingKind, USER_EMAIL};
use libtest_mimic::{Failed, Trial};

/// The route under test.
const ROUTE: Route = Route::FollowupEmail;
/// What the draft must mention: the call's main outcome.
const KEY_FACT: &str = "pilot";
/// The external guests' first names, one of which the greeting uses.
const GUESTS: [&str; 2] = ["Marcus", "Hannah"];
/// The colleague's promise appended to the `colleague_owns` transcript.
const COLLEAGUE_PROMISE: &str =
    "Lena Okafor: I'll send the recap and the pilot proposal to Marcus and Hannah myself tonight.";

/// One follow-up case: the meeting both calls read, and the verdict a good
/// decision reaches.
#[derive(Clone)]
struct FollowUpCase {
    name: &'static str,
    decide: Prompt,
    draft: Prompt,
    expect_send: bool,
}

/// Every follow-up trial.
pub fn trials(live_enabled: bool) -> Vec<Trial> {
    let cases = match cases() {
        Ok(cases) => cases,
        Err(error) => return vec![Trial::test("setup::followup", move || Err(error))],
    };
    cases
        .into_iter()
        .flat_map(|case| {
            let (contract, smoke, live) = (case.clone(), case.clone(), case);
            vec![
                Trial::test(format!("contract::{ROUTE}::{}", contract.name), move || {
                    check_prompts(&contract)
                }),
                Trial::test(format!("smoke::{ROUTE}::{}", smoke.name), move || {
                    run_smoke(&smoke)
                }),
                Trial::test(format!("live::{ROUTE}::{}", live.name), move || {
                    run_live(&live)
                })
                .with_ignored_flag(!live_enabled),
            ]
        })
        .collect()
}

fn cases() -> Result<Vec<FollowUpCase>, Failed> {
    let fixtures = Fixtures::load().map_err(|error| Failed::from(error.to_string()))?;
    let meeting = fixtures
        .meeting(MeetingKind::SalesDiscovery)
        .ok_or("the sales discovery fixture is missing")?;
    let header = format!(
        "Title: {}\nDate: Wednesday 30 September 2026",
        meeting.title
    );
    let people = people(meeting);
    let notes = meeting.user_notes.clone().unwrap_or_default();
    let transcript = transcript(meeting);
    let owned = format!("{transcript}\n{COLLEAGUE_PROMISE}");
    let case = |name, transcript: &str, expect_send| {
        let input = |task| {
            FollowUpUser {
                meeting: &header,
                people: &people,
                notes: &notes,
                transcript,
                task,
            }
            .text()
        };
        FollowUpCase {
            name,
            decide: Prompt::new(input(FOLLOW_UP_DECIDE_TASK))
                .system(FollowUpDecideSystem.text())
                .max_tokens(512),
            draft: Prompt::new(input(FOLLOW_UP_DRAFT_TASK))
                .system(FollowUpDraftSystem.text())
                .max_tokens(1_024),
            expect_send,
        }
    };
    Ok(vec![
        case("sales_discovery", &transcript, true),
        case("colleague_owns", &owned, false),
    ])
}

/// `- Name (me)`, `- Name` or `- Name (external)`, as the core marks them.
fn people(meeting: &FixtureMeeting) -> String {
    let lines: Vec<String> = meeting
        .attendees
        .iter()
        .map(|person| {
            let mark = match person.email.as_str() {
                USER_EMAIL => " (me)",
                email if email.ends_with("@deino.dev") => "",
                _ => " (external)",
            };
            format!("- {}{mark}", person.name)
        })
        .collect();
    lines.join("\n")
}

/// `Name: text` lines, each voice named after its attendee.
fn transcript(meeting: &FixtureMeeting) -> String {
    let name = |label: &str| {
        let speaker = meeting.speakers.iter().find(|s| s.label == label);
        let email = speaker.and_then(|speaker| speaker.attendee.as_ref());
        let attendee = email.and_then(|e| meeting.attendees.iter().find(|a| &a.email == e));
        attendee.map_or_else(|| label.to_owned(), |attendee| attendee.name.clone())
    };
    let lines: Vec<String> = meeting
        .segments
        .iter()
        .map(|segment| format!("{}: {}", name(&segment.speaker), segment.text))
        .collect();
    lines.join("\n")
}

fn user_text(prompt: &Prompt) -> String {
    prompt
        .messages
        .first()
        .map(deino_llm::Message::text)
        .unwrap_or_default()
}

fn check_prompts(case: &FollowUpCase) -> Result<(), Failed> {
    let decide = case.decide.system.clone().unwrap_or_default();
    let draft = case.draft.system.clone().unwrap_or_default();
    if !decide.contains("colleague_owns") || !decide.contains("moved_to_chat") {
        return Err("the decision lost a reason to stay quiet".into());
    }
    if !draft.contains("square brackets") {
        return Err("the draft lost the placeholder instruction".into());
    }
    let user = user_text(&case.draft);
    for guest in GUESTS {
        if !user.contains(&format!("- {guest}")) || !user.contains("(external)") {
            return Err(format!("the prompt does not mark {guest} external").into());
        }
    }
    if !user.contains("(me)") {
        return Err("the prompt does not mark the user".into());
    }
    Ok(())
}

/// Whether `decision` is the right call for `case`.
fn judge_decision(case: &FollowUpCase, decision: &FollowUpDecision) -> Result<(), String> {
    let sends = decision.verdict == FollowUpVerdict::Send;
    (sends == case.expect_send).then_some(()).ok_or_else(|| {
        format!(
            "expected send = {}, got {:?}",
            case.expect_send, decision.verdict
        )
    })
}

/// Whether `draft` is a good follow-up of the discovery call.
fn judge_draft(draft: &FollowUpDraftAnswer) -> Result<(), String> {
    if draft.subject.trim().is_empty() || draft.subject.chars().count() > 80 {
        return Err(format!(
            "bad subject length: {}",
            draft.subject.chars().count()
        ));
    }
    let body = draft.body.to_lowercase();
    if !GUESTS
        .iter()
        .any(|guest| body.contains(&guest.to_lowercase()))
    {
        return Err("the draft greets neither guest".to_owned());
    }
    if !body.contains(KEY_FACT) {
        return Err(format!("the draft misses the {KEY_FACT}"));
    }
    if placeholders(&draft.body).is_empty() {
        return Err("the draft has no [placeholder]".to_owned());
    }
    Ok(())
}

fn good_decision(case: &FollowUpCase) -> FollowUpDecision {
    let verdict = if case.expect_send {
        FollowUpVerdict::Send
    } else {
        FollowUpVerdict::ColleagueOwns
    };
    FollowUpDecision {
        reason: "Next steps were agreed.".into(),
        verdict,
    }
}

fn good_draft() -> FollowUpDraftAnswer {
    FollowUpDraftAnswer {
        subject: "Brightkite pilot: next steps".into(),
        body: "Hi Marcus and Hannah,\n\nThanks for your time today. We agreed on a 60-day pilot in one region.\n\nI'll send the proposal by [proposal date].\n\nShreyas".into(),
    }
}

fn run_smoke(case: &FollowUpCase) -> Result<(), Failed> {
    let mut provider = MockProvider::new().reply(ROUTE, &json(&good_decision(case))?);
    if case.expect_send {
        provider = provider.reply(ROUTE, &json(&good_draft())?);
    }
    let gateway = mock_gateway(Arc::new(provider)).gateway;
    judge(case, &gateway, None)
}

fn json(value: &impl serde::Serialize) -> Result<String, Failed> {
    serde_json::to_string(value).map_err(|error| Failed::from(error.to_string()))
}

fn run_live(case: &FollowUpCase) -> Result<(), Failed> {
    let gateway = crate::live::gateway_for(ROUTE)?;
    for _ in 0..crate::live::samples() {
        judge(case, &gateway, crate::live::target())?;
    }
    Ok(())
}

/// Asks for the decision and, when it should be `send`, the draft, and
/// judges both.
fn judge(
    case: &FollowUpCase,
    gateway: &Gateway,
    target: Option<deino_llm::Target>,
) -> Result<(), Failed> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| Failed::from(e.to_string()))?;
    let mut decide = case.decide.clone();
    decide.target = target.clone();
    let decision: FollowUpDecision = runtime
        .block_on(gateway.structured(ROUTE, decide))
        .map_err(|error| Failed::from(error.to_string()))?;
    judge_decision(case, &decision).map_err(Failed::from)?;
    if !case.expect_send {
        return Ok(());
    }
    let mut draft = case.draft.clone();
    draft.target = target;
    let answer: FollowUpDraftAnswer = runtime
        .block_on(gateway.structured(ROUTE, draft))
        .map_err(|error| Failed::from(error.to_string()))?;
    judge_draft(&answer).map_err(Failed::from)
}
