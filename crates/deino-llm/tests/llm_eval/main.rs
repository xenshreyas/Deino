//! LLM evals: route prompts scored against the fixture meetings' ground truth
//! (PARITY_PLAN §3A "Testing"; the anarlog `template-eval` harness pattern, MIT).
//!
//! Every case yields three trials:
//!
//! - `contract::<route>::<case>`: the prompt renders and carries what the
//!   model needs (offline, always runs);
//! - `smoke::<route>::<case>`: the expectations pass on a known-good answer
//!   through `MockProvider`, so the harness itself is tested (offline);
//! - `live::<route>::<case>`: a real provider, `DEINO_EVAL_SAMPLES` times
//!   (default 1), passing when the share of samples that meet every
//!   expectation reaches the case's bar. Ignored unless `DEINO_EVAL=1`.
//!
//! `extract.action_items` (AI-05) scores the extracted items of all eight
//! fixtures against their ground truth (`action_items.rs`, `scoring.rs`).
//!
//! ```sh
//! DEINO_EVAL=1 ANTHROPIC_API_KEY=… pnpm eval title     # from frontend/
//! DEINO_EVAL=1 ANTHROPIC_API_KEY=… pnpm eval extract.action_items
//! DEINO_EVAL=1 cargo test -p deino-llm --test llm_eval -- title
//! ```
//!
//! `DEINO_EVAL_TARGET=openai:gpt-5-mini` serves the live trials from another
//! provider (its key in `OPENAI_API_KEY`). Evals never run in CI.

mod action_items;
mod cases;
mod expectations;
mod live;
mod scoring;

use std::sync::Arc;

use deino_llm::{MockProvider, mock_gateway};
use libtest_mimic::{Arguments, Failed, Trial};

use crate::cases::Case;

fn main() {
    let args = Arguments::from_args();
    let mut trials: Vec<Trial> = match cases::all() {
        Ok(cases) => cases.into_iter().flat_map(trials).collect(),
        Err(error) => vec![Trial::test("setup::load_fixtures", move || {
            Err(error.into())
        })],
    };
    trials.extend(action_items::trials(live_enabled()));
    libtest_mimic::run(&args, trials).exit();
}

/// True when `DEINO_EVAL=1` asks for the live trials.
fn live_enabled() -> bool {
    std::env::var("DEINO_EVAL").is_ok_and(|value| value == "1")
}

fn trials(case: Case) -> Vec<Trial> {
    let live_enabled = live_enabled();
    let name = format!("{}::{}", case.route, case.name);
    let (contract, smoke, live) = (case.clone(), case.clone(), case);
    vec![
        Trial::test(format!("contract::{name}"), move || contract.check_prompt()),
        Trial::test(format!("smoke::{name}"), move || run_smoke(&smoke)),
        Trial::test(format!("live::{name}"), move || live::run(&live))
            .with_ignored_flag(!live_enabled),
    ]
}

/// The case's known-good answer through the real gateway and `MockProvider`.
fn run_smoke(case: &Case) -> Result<(), Failed> {
    let runtime =
        tokio::runtime::Runtime::new().map_err(|error| Failed::from(error.to_string()))?;
    let harness = mock_gateway(Arc::new(
        MockProvider::new().reply(case.route, &case.smoke_answer),
    ));
    let completion = runtime
        .block_on(harness.gateway.complete(case.route, case.prompt.clone()))
        .map_err(|error| Failed::from(error.to_string()))?;
    expectations::check(&case.expectations, &completion.text).map_err(Failed::from)
}
