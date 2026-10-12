//! `brief` (NOT-08): the pre-meeting brief prompt over two cases built from
//! the fixtures, the way `deino_core::briefs` builds it (numbered source
//! blocks, absolute dates).
//!
//! - `follow_up`: a follow-up with the sales discovery fixture's guests; the
//!   discovery call's notes are `m1`, the invitation `e1`. A good brief has
//!   two to four bullets, each citing only `m1` or `e1`, and names the
//!   licence deadline (the call's key fact).
//! - `new_contact`: someone never met; only the invitation. A good answer
//!   says `insufficient`.
//!
//! Trials: `contract::brief::<case>` (the prompt carries the rules, the
//! sources and their references), `smoke::brief::<case>` (a known-good
//! answer through `MockProvider` and `Gateway::structured` passes the
//! checks), `live::brief::<case>` (a real provider; ignored unless
//! `DEINO_EVAL=1`).

use std::sync::Arc;

use deino_llm::brief::{BriefAnswer, BriefAnswerBullet, SourceKind, SourceRef, source_block};
use deino_llm::prompts::{BriefSystem, BriefUser, PromptText};
use deino_llm::{Gateway, MockProvider, Prompt, Route, mock_gateway};
use deino_testkit::fixture::{Fixtures, MeetingKind};
use libtest_mimic::{Failed, Trial};

/// The route under test.
const ROUTE: Route = Route::Brief;
/// What the follow-up brief must mention: the discovery call's deadline.
const KEY_FACT: &str = "january";

/// One brief case.
#[derive(Clone)]
struct BriefCase {
    name: &'static str,
    prompt: Prompt,
    /// The references the prompt shows.
    references: Vec<String>,
    /// Whether a good answer is `insufficient`.
    expect_insufficient: bool,
}

/// Every brief trial.
pub fn trials(live_enabled: bool) -> Vec<Trial> {
    let cases = match cases() {
        Ok(cases) => cases,
        Err(error) => return vec![Trial::test("setup::brief", move || Err(error))],
    };
    cases
        .into_iter()
        .flat_map(|case| {
            let (contract, smoke, live) = (case.clone(), case.clone(), case);
            vec![
                Trial::test(format!("contract::{ROUTE}::{}", contract.name), move || {
                    check_prompt(&contract)
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

fn reference(kind: SourceKind) -> SourceRef {
    SourceRef { kind, number: 1 }
}

fn cases() -> Result<Vec<BriefCase>, Failed> {
    let fixtures = Fixtures::load().map_err(|error| Failed::from(error.to_string()))?;
    let discovery = fixtures
        .meeting(MeetingKind::SalesDiscovery)
        .ok_or("the sales discovery fixture is missing")?;
    let invitation = "Agenda: pilot scope, pricing and next steps.";
    let meeting = source_block(
        reference(SourceKind::Meeting),
        &format!("Meeting: {} (Wednesday 30 September 2026)", discovery.title),
        discovery.user_notes.as_deref().unwrap_or_default(),
    );
    let event = source_block(reference(SourceKind::Event), "Invitation", invitation);
    let follow_up = prompt(
        "Title: Brightkite follow-up\nWhen: Monday 12 October 2026, 10:00\nAttendees:\n- Marcus Webb, Brightkite Logistics",
        &format!("{meeting}\n\n{event}"),
    );
    let new_contact = prompt(
        "Title: Intro call\nWhen: Monday 12 October 2026, 11:00\nAttendees:\n- ines@newco.example",
        &event,
    );
    Ok(vec![
        BriefCase {
            name: "follow_up",
            prompt: follow_up,
            references: vec!["m1".into(), "e1".into()],
            expect_insufficient: false,
        },
        BriefCase {
            name: "new_contact",
            prompt: new_contact,
            references: vec!["e1".into()],
            expect_insufficient: true,
        },
    ])
}

fn prompt(event: &str, sources: &str) -> Prompt {
    Prompt::new(BriefUser { event, sources }.text())
        .system(BriefSystem.text())
        .max_tokens(1_024)
}

fn check_prompt(case: &BriefCase) -> Result<(), Failed> {
    let system = case.prompt.system.clone().unwrap_or_default();
    let user = case
        .prompt
        .messages
        .first()
        .map(|m| m.text())
        .unwrap_or_default();
    if !system.contains("insufficient") || !system.contains("cites") {
        return Err("the rules lost the citation or insufficient instruction".into());
    }
    for reference in &case.references {
        if !user.contains(&format!("[{reference}]")) {
            return Err(format!("the prompt does not show [{reference}]").into());
        }
    }
    Ok(())
}

/// Whether `answer` is a good brief for `case`.
fn judge(case: &BriefCase, answer: &BriefAnswer) -> Result<(), String> {
    if case.expect_insufficient {
        return answer
            .insufficient
            .then_some(())
            .ok_or_else(|| "expected `insufficient` for a new contact".to_owned());
    }
    if answer.insufficient || !(2..=4).contains(&answer.bullets.len()) {
        return Err(format!(
            "expected 2–4 bullets, got {}",
            answer.bullets.len()
        ));
    }
    let known = |source: &String| {
        SourceRef::parse(source).is_some_and(|parsed| case.references.contains(&parsed.label()))
    };
    if let Some(bullet) = answer
        .bullets
        .iter()
        .find(|b| b.sources.is_empty() || !b.sources.iter().all(known))
    {
        return Err(format!(
            "a bullet cites nothing or something unknown: {:?}",
            bullet.sources
        ));
    }
    let text = answer
        .bullets
        .iter()
        .map(|b| b.text.to_lowercase())
        .collect::<String>();
    text.contains(KEY_FACT)
        .then_some(())
        .ok_or_else(|| format!("the brief misses the deadline ({KEY_FACT})"))
}

fn good_answer(case: &BriefCase) -> BriefAnswer {
    if case.expect_insufficient {
        return BriefAnswer {
            insufficient: true,
            bullets: Vec::new(),
        };
    }
    let bullet = |text: &str, source: &str| BriefAnswerBullet {
        text: text.into(),
        sources: vec![source.into()],
    };
    BriefAnswer {
        insufficient: false,
        bullets: vec![
            bullet(
                "Their DispatchPro licence ends on 31 January 2027, a hard deadline.",
                "[m1]",
            ),
            bullet(
                "The agenda covers pilot scope, pricing and next steps.",
                "e1",
            ),
        ],
    }
}

fn run_smoke(case: &BriefCase) -> Result<(), Failed> {
    let json =
        serde_json::to_string(&good_answer(case)).map_err(|e| Failed::from(e.to_string()))?;
    let gateway = mock_gateway(Arc::new(MockProvider::new().reply(ROUTE, &json))).gateway;
    let answer = block_on(&gateway, case.prompt.clone())?;
    judge(case, &answer).map_err(Failed::from)
}

fn run_live(case: &BriefCase) -> Result<(), Failed> {
    let gateway = crate::live::gateway_for(ROUTE)?;
    for _ in 0..crate::live::samples() {
        let mut prompt = case.prompt.clone();
        prompt.target = crate::live::target();
        judge(case, &block_on(&gateway, prompt)?).map_err(Failed::from)?;
    }
    Ok(())
}

fn block_on(gateway: &Gateway, prompt: Prompt) -> Result<BriefAnswer, Failed> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| Failed::from(e.to_string()))?;
    runtime
        .block_on(gateway.structured::<BriefAnswer>(ROUTE, prompt))
        .map_err(|error| Failed::from(error.to_string()))
}
