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

use std::sync::Arc;

use deino_llm::{Gateway, LlmError, Prompt, ProviderId, Route, Target};
use futures_util::{StreamExt, TryStreamExt, stream};

use super::citations::SegmentIndex;
use super::material::Material;
use super::prompt::PromptInputs;
use super::transcript::{PART_SPAN, PartLimits, TranscriptLine, parts};
use super::wire::EnhancedDoc;
use crate::templates::Template;

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
        let cuts = parts(&material.transcript, self.limits(&target).await);
        let answer = if cuts.len() <= 1 {
            self.ask(inputs.single(&material.transcript), &target)
                .await?
        } else {
            let part_notes = self.map(&inputs, &cuts, &target).await?;
            let noted: Vec<(&[TranscriptLine], EnhancedDoc)> =
                cuts.iter().copied().zip(part_notes).collect();
            self.ask(inputs.merge(&noted, index), &target).await?
        };
        log::info!(target: "enhance", "{}: wrote notes in {} part(s)", material.meeting.id, cuts.len().max(1));
        Ok(Written {
            answer,
            model: model_label(&target),
            parts: cuts.len().max(1),
        })
    }

    /// The map step: each part's notes, in order.
    async fn map(
        &self,
        inputs: &PromptInputs,
        cuts: &[&[TranscriptLine]],
        target: &Target,
    ) -> Result<Vec<EnhancedDoc>, LlmError> {
        let count = cuts.len();
        // Collected first: a lazy iterator's closure inside the stream would
        // make the future `Send` only for one lifetime (rust-lang/rust#102211).
        let calls: Vec<_> = cuts
            .iter()
            .enumerate()
            .map(|(at, lines)| self.ask(inputs.part(at + 1, count, lines), target))
            .collect();
        stream::iter(calls)
            .buffered(concurrency(target))
            .try_collect()
            .await
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
