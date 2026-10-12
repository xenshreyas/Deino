//! The light LLM check: [`SpanCheck`] confirms or rejects the candidates
//! the patterns found from context alone ([`Confidence::Likely`]).
//!
//! It only ever answers yes or no per candidate; it cannot add spans. It
//! sees one short excerpt around each candidate ([`excerpt`]), never the
//! transcript. [`LocalModelCheck`] asks the built-in local model and does
//! nothing when the route would leave the Mac (ADR 0038): sending a
//! suspected password to a cloud model to ask whether to hide it from a
//! cloud model would defeat the point. Every failure keeps the candidate,
//! so the check can only make redaction narrower, never leak more.
//!
//! [`Confidence::Likely`]: super::Confidence::Likely

use std::sync::Arc;

use async_trait::async_trait;
use deino_llm::prompts::{PromptText, RedactCheckSystem, RedactCheckUser};
use deino_llm::{Gateway, Prompt, ProviderId, Route};
use schemars::JsonSchema;
use serde::Deserialize;

use super::model::SecretKind;
use super::words::words;

/// Words of context shown before a candidate.
const WORDS_BEFORE: usize = 6;
/// Words of context shown after a candidate.
const WORDS_AFTER: usize = 3;
/// Room for one short verdict per candidate.
const TOKENS_PER_CANDIDATE: u32 = 24;
/// The route the check is metered under: short local classification.
const ROUTE: Route = Route::Tagging;

/// A candidate as the check sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// What the pattern took it for.
    pub kind: SecretKind,
    /// A few words around it, the candidate between « and ».
    pub excerpt: String,
}

/// Confirms or rejects candidate secrets.
///
/// Contract: returns one verdict per candidate, in order, `true` to
/// redact. Anything it cannot judge is `true`.
#[async_trait]
pub trait SpanCheck: Send + Sync {
    /// The verdicts on `candidates`.
    async fn confirm(&self, candidates: &[Candidate]) -> Vec<bool>;
}

/// No check (regex only): every candidate is redacted.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoCheck;

#[async_trait]
impl SpanCheck for NoCheck {
    async fn confirm(&self, candidates: &[Candidate]) -> Vec<bool> {
        vec![true; candidates.len()]
    }
}

/// [`SpanCheck`] over the built-in local model, through the gateway on
/// the `tagging` route. When that route is served by a hosted model, or no
/// local model answers, every candidate is kept as a secret.
pub struct LocalModelCheck {
    gateway: Arc<Gateway>,
}

impl LocalModelCheck {
    /// A check over `gateway`.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl SpanCheck for LocalModelCheck {
    async fn confirm(&self, candidates: &[Candidate]) -> Vec<bool> {
        let all = vec![true; candidates.len()];
        let local = match self.gateway.target(ROUTE, &Prompt::default()).await {
            Ok(target) if target.provider == ProviderId::BuiltIn && !target.model.is_empty() => {
                target
            }
            _ => return all,
        };
        let budget = TOKENS_PER_CANDIDATE
            .saturating_mul(u32::try_from(candidates.len()).unwrap_or(u32::MAX));
        let prompt = Prompt::new(
            RedactCheckUser {
                candidates: &listing(candidates),
            }
            .text(),
        )
        .system(RedactCheckSystem.text())
        .max_tokens(budget)
        .on(local);
        match self.gateway.structured::<Verdicts>(ROUTE, prompt).await {
            Ok(verdicts) => verdicts.apply(candidates.len()),
            Err(error) => {
                log::warn!(target: "redaction", "the local check failed ({error}); redacting all {} candidates", candidates.len());
                all
            }
        }
    }
}

/// The check's structured answer.
#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct Verdicts {
    /// One verdict per numbered candidate.
    verdicts: Vec<Verdict>,
}

/// Whether one candidate is a secret.
#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct Verdict {
    /// The candidate's number, as listed.
    number: usize,
    /// True when it reveals a secret.
    secret: bool,
}

impl Verdicts {
    /// One verdict per candidate: only an explicit "not a secret" rejects.
    pub(super) fn apply(&self, count: usize) -> Vec<bool> {
        let mut keep = vec![true; count];
        for verdict in self.verdicts.iter().filter(|verdict| !verdict.secret) {
            if let Some(slot) = verdict
                .number
                .checked_sub(1)
                .and_then(|at| keep.get_mut(at))
            {
                *slot = false;
            }
        }
        keep
    }
}

/// `n. [kind] excerpt`, one line per candidate.
fn listing(candidates: &[Candidate]) -> String {
    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            format!(
                "{}. [{}] {}",
                index + 1,
                candidate.kind.label(),
                candidate.excerpt
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The words around `range` in `text`, the span itself between « and ».
pub(super) fn excerpt(text: &str, range: std::ops::Range<usize>) -> String {
    let all = words(text);
    let before = all
        .iter()
        .filter(|word| word.range.end <= range.start)
        .count();
    let start = before
        .checked_sub(WORDS_BEFORE)
        .and_then(|at| all.get(at))
        .map_or(0, |word| word.range.start);
    let end = all
        .iter()
        .filter(|word| word.range.start >= range.end)
        .nth(WORDS_AFTER.saturating_sub(1))
        .map_or(text.len(), |word| word.range.end);
    let slice = |from: usize, to: usize| text.get(from..to).unwrap_or_default();
    format!(
        "{}«{}»{}",
        slice(start, range.start),
        slice(range.start, range.end),
        slice(range.end, end)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_excerpt_is_a_few_words_around_the_candidate() {
        let text = "one two three four five six seven the code is 4821 and then we left the call";
        let start = text.find("4821").unwrap_or_default();

        let shown = excerpt(text, start..start + 4);

        assert_eq!(shown, "five six seven the code is «4821» and then we");
    }

    #[test]
    fn only_an_explicit_no_rejects() {
        let verdicts = Verdicts {
            verdicts: vec![
                Verdict {
                    number: 2,
                    secret: false,
                },
                Verdict {
                    number: 3,
                    secret: true,
                },
                Verdict {
                    number: 9,
                    secret: false,
                },
            ],
        };

        assert_eq!(verdicts.apply(4), [true, false, true, true]);
    }
}
