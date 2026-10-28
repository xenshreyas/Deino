//! [`NotesWriter`]: the `notes.enhance` calls, one pass or map-reduce.
//!
//! A meeting of up to [`LONG_MEETING`](super::transcript::LONG_MEETING)
//! whose transcript fits the model is written in one call. A longer one is
//! cut into parts of about 20 minutes (`transcript::parts`), each part
//! noted in parallel (map, at most [`MAP_CONCURRENCY`] calls at once on a
//! hosted model, one at a time on a local one), then the parts' notes are
//! merged with the user's notes and the template (reduce). Every call is
//! pinned to the model the route resolved to at the start, so a settings
//! change mid-run cannot mix models. Dropping the future cancels every call
//! in flight.
//!
//! An answer that cannot be used (cut off at the token cap, or JSON that
//! does not parse even after the gateway's repair) does not sink the notes:
//! the call is asked again, a part that still fails is noted in halves and a
//! half that fails is left out, a one-pass meeting that fails is noted in
//! halves, and a merge that fails is done here ([`fallback::or_merge_locally`]).
//! Only a request the provider refuses (no key, auth, rate limits, ...)
//! fails the notes, or every part failing.

use std::sync::Arc;

use deino_llm::{Gateway, LlmError, Prompt, ProviderId, Route, Target};
use futures_util::{StreamExt, TryStreamExt, stream};

use super::citations::SegmentIndex;
use super::material::Material;
use super::prompt::PromptInputs;
use super::transcript::{PART_SPAN, PartLimits, TranscriptLine, parts};
use super::wire::EnhancedDoc;
use crate::templates::Template;

mod fallback;

use fallback::{halve, is_answer_failure, or_merge_locally};

/// Parts noted at once on a hosted model: fast, and well inside rate limits.
pub const MAP_CONCURRENCY: usize = 4;
/// Transcript characters per call when the model does not say its context
/// window (hosted models, which all take far more): about 30k tokens.
const HOSTED_PART_CHARS: usize = 120_000;
/// Tokens of a local model's window kept for the prompt around the
/// transcript and for the answer.
const RESERVED_TOKENS: u32 = 6_000;
/// Characters per token, conservatively, for English transcripts.
const CHARS_PER_TOKEN: usize = 3;
/// The smallest part a tiny context window still gets.
const MIN_PART_CHARS: usize = 2_000;

/// What the model wrote, and with what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    /// The notes as the model answered (or merged) them.
    pub answer: EnhancedDoc,
    /// `provider:model`.
    pub model: String,
    /// How many transcript parts were noted (1 for one pass).
    pub parts: usize,
}

/// Part `number` (from 1) of `count` of a long meeting's transcript.
#[derive(Clone, Copy)]
struct Part<'a> {
    number: usize,
    count: usize,
    lines: &'a [TranscriptLine],
}

/// Writes notes through the gateway.
pub struct NotesWriter {
    gateway: Arc<Gateway>,
}

impl NotesWriter {
    /// A writer over `gateway`.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }

    /// The notes of `material`, written with `template`.
    pub async fn write(
        &self,
        material: &Material,
        template: &Template,
        index: &SegmentIndex,
    ) -> Result<Written, LlmError> {
        let inputs = PromptInputs::new(material, template);
        let target = self
            .gateway
            .target(Route::NotesEnhance, &Prompt::default())
            .await?;
        let mut cuts = parts(&material.transcript, self.limits(&target).await);
        if cuts.len() <= 1 {
            match self
                .one_pass(&inputs, &material.transcript, &target)
                .await?
            {
                Some(answer) => return Ok(written(material, answer, &target, 1)),
                None => cuts = halve(&material.transcript),
            }
        }
        let noted = self.map(&inputs, &cuts, &target).await?;
        let merged = self.ask_again(inputs.merge(&noted, index), &target).await;
        let answer = or_merge_locally(merged, &noted)?;
        Ok(written(material, answer, &target, cuts.len()))
    }

    /// The notes of a short meeting in one call, or `None` when its answer
    /// could not be used and the transcript can be noted in halves instead.
    async fn one_pass(
        &self,
        inputs: &PromptInputs,
        lines: &[TranscriptLine],
        target: &Target,
    ) -> Result<Option<EnhancedDoc>, LlmError> {
        match self.ask_again(inputs.single(lines), target).await {
            Ok(answer) => Ok(Some(answer)),
            Err(error) if is_answer_failure(&error) && lines.len() >= 2 => {
                log::warn!(target: "enhance", "one pass failed ({:?}); noting in halves", error.kind());
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// The map step: each part's notes, in order, a part that failed noted
    /// in halves; at least one part must be noted.
    async fn map<'a>(
        &self,
        inputs: &PromptInputs,
        cuts: &[&'a [TranscriptLine]],
        target: &Target,
    ) -> Result<Vec<(&'a [TranscriptLine], EnhancedDoc)>, LlmError> {
        let count = cuts.len();
        // Collected first: a lazy iterator's closure inside the stream would
        // make the future `Send` only for one lifetime (rust-lang/rust#102211).
        let calls: Vec<_> = cuts
            .iter()
            .enumerate()
            .map(|(at, lines)| {
                let part = Part {
                    number: at + 1,
                    count,
                    lines,
                };
                self.note_part(inputs, part, target)
            })
            .collect();
        let noted: Vec<Vec<_>> = stream::iter(calls)
            .buffered(concurrency(target))
            .try_collect()
            .await?;
        let noted: Vec<_> = noted.into_iter().flatten().collect();
        if noted.is_empty() {
            return Err(LlmError::InvalidOutput {
                detail: "no part of the transcript could be noted".to_owned(),
            });
        }
        Ok(noted)
    }

    /// A part's notes, or its halves' notes when its answer could not be
    /// used twice; a half that fails is left out.
    async fn note_part<'a>(
        &self,
        inputs: &PromptInputs,
        part: Part<'a>,
        target: &Target,
    ) -> Result<Vec<(&'a [TranscriptLine], EnhancedDoc)>, LlmError> {
        let Part {
            number,
            count,
            lines,
        } = part;
        match self
            .ask_again(inputs.part(number, count, lines), target)
            .await
        {
            Ok(whole) => return Ok(vec![(lines, whole)]),
            Err(error) if !is_answer_failure(&error) => return Err(error),
            Err(error) => {
                log::warn!(target: "enhance", "part {number} of {count} failed ({:?}); noting it in halves", error.kind());
            }
        }
        let mut kept = Vec::new();
        for half in halve(lines) {
            match self.ask(inputs.part(number, count, half), target).await {
                Ok(half_notes) => kept.push((half, half_notes)),
                Err(error) if is_answer_failure(&error) => {
                    log::warn!(target: "enhance", "half of part {number} of {count} left out ({:?})", error.kind());
                }
                Err(error) => return Err(error),
            }
        }
        Ok(kept)
    }

    /// [`Self::ask`], asked once more when the answer could not be used:
    /// the model samples afresh, and a loop or a cut-off rarely repeats.
    async fn ask_again(&self, prompt: Prompt, target: &Target) -> Result<EnhancedDoc, LlmError> {
        match self.ask(prompt.clone(), target).await {
            Err(error) if is_answer_failure(&error) => {
                log::info!(target: "enhance", "notes answer unusable ({:?}); asking again", error.kind());
                self.ask(prompt, target).await
            }
            other => other,
        }
    }

    async fn ask(&self, prompt: Prompt, target: &Target) -> Result<EnhancedDoc, LlmError> {
        self.gateway
            .structured::<EnhancedDoc>(Route::NotesEnhance, prompt.on(target.clone()))
            .await
    }

    /// How to cut the transcript for `target`: its context window when it
    /// reports one (local models), else the hosted budget.
    async fn limits(&self, target: &Target) -> PartLimits {
        let chars = match self.gateway.context_window(target).await {
            Some(window) => usize::try_from(window.saturating_sub(RESERVED_TOKENS))
                .unwrap_or(usize::MAX)
                .saturating_mul(CHARS_PER_TOKEN)
                .max(MIN_PART_CHARS),
            None => HOSTED_PART_CHARS,
        };
        PartLimits {
            span: PART_SPAN,
            chars,
        }
    }
}

/// What was written for `material`, from `parts` part(s).
fn written(material: &Material, answer: EnhancedDoc, target: &Target, parts: usize) -> Written {
    log::info!(target: "enhance", "{}: wrote notes in {} part(s)", material.meeting.id, parts.max(1));
    Written {
        answer,
        model: model_label(target),
        parts: parts.max(1),
    }
}

/// One call at a time on a local model (one GPU), several on a hosted one.
fn concurrency(target: &Target) -> usize {
    match target.provider {
        ProviderId::BuiltIn | ProviderId::Ollama => 1,
        _ => MAP_CONCURRENCY,
    }
}

/// `anthropic:claude-sonnet-5-5`.
pub fn model_label(target: &Target) -> String {
    let provider = serde_json::to_value(target.provider)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();
    format!("{provider}:{}", target.model)
}
