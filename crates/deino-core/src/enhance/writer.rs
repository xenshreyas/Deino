//! [`NotesWriter`]: the `notes.enhance` calls, one pass or map-reduce.
//!
//! A transcript that fits the model (about 30k tokens on a hosted model) is
//! written in one call, however long the meeting ran. A longer one is cut
//! into parts of about 20 minutes (`transcript::parts`), each part
//! noted in parallel (map, at most [`MAP_CONCURRENCY`] calls at once on a
//! hosted model, one at a time on a local one), then the parts' notes are
//! merged with the user's notes and the template (reduce). Every call is
//! pinned to the model the route resolved to at the start, so a settings
//! change mid-run cannot mix models. Dropping the future cancels every call
//! in flight.
//!
//! The calls whose answer becomes the notes (one pass, the merge) stream it
//! to the run's [`WriteReport`], which shows the notes as they are written;
//! the parts only report how many are done.
//!
//! An answer that cannot be used (cut off at the token cap, or JSON that
//! does not parse even after the gateway's repair) does not sink the notes:
//! the call is asked again, a part that still fails is noted in halves and a
//! half that fails is left out, a one-pass meeting that fails is noted in
//! halves, and a merge that fails is done here ([`fallback::or_merge_locally`]).
//! Only a request the provider refuses (no key, auth, rate limits, ...)
//! fails the notes, or every part failing.

use std::sync::Arc;

use deino_llm::{AnswerSink, Gateway, LlmError, Prompt, ProviderId, Route, Target};

use super::citations::SegmentIndex;
use super::material::Material;
use super::prompt::PromptInputs;
use super::transcript::{PART_SPAN, PartLimits, TranscriptLine, parts};
use super::wire::EnhancedDoc;
use crate::templates::Template;

mod fallback;
mod map;

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

/// What one run of the writer is given.
pub struct WriteJob<'a> {
    /// What the notes are written from.
    pub material: &'a Material,
    /// The meeting's template.
    pub template: &'a Template,
    /// The transcript's references.
    pub index: &'a SegmentIndex,
    /// Where the run reports how far it got.
    pub report: &'a dyn WriteReport,
}

/// How far a run got: the notes' answer as it streams, and the parts.
pub trait WriteReport: Send + Sync {
    /// Receives the answer of each call that becomes the notes.
    fn sink(&self) -> &dyn AnswerSink;
    /// `done` of `total` parts of a long transcript are noted.
    fn parts(&self, done: usize, total: usize);
    /// The parts are noted; the merge call starts.
    fn merging(&self);
}

/// The calls of one run: its prompts, pinned target and report.
struct Pass<'a> {
    inputs: PromptInputs,
    target: Target,
    report: &'a dyn WriteReport,
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

    /// The notes of `job.material`, written with `job.template`.
    pub async fn write(&self, job: &WriteJob<'_>) -> Result<Written, LlmError> {
        let material = job.material;
        let pass = Pass {
            inputs: PromptInputs::new(material, job.template),
            target: self
                .gateway
                .target(Route::NotesEnhance, &Prompt::default())
                .await?,
            report: job.report,
        };
        let mut cuts = parts(&material.transcript, self.limits(&pass.target).await);
        if cuts.len() <= 1 {
            match self.one_pass(&pass, &material.transcript).await? {
                Some(answer) => return Ok(written(material, answer, &pass.target, 1)),
                None => cuts = halve(&material.transcript),
            }
        }
        let noted = self.map(&pass, &cuts).await?;
        pass.report.merging();
        let merge = pass.inputs.merge(&noted, job.index);
        let merged = self
            .ask_again(merge, &pass.target, Some(pass.report.sink()))
            .await;
        let answer = or_merge_locally(merged, &noted)?;
        Ok(written(material, answer, &pass.target, cuts.len()))
    }

    /// The notes of a short meeting in one call, or `None` when its answer
    /// could not be used and the transcript can be noted in halves instead.
    async fn one_pass(
        &self,
        pass: &Pass<'_>,
        lines: &[TranscriptLine],
    ) -> Result<Option<EnhancedDoc>, LlmError> {
        let sink = Some(pass.report.sink());
        match self
            .ask_again(pass.inputs.single(lines), &pass.target, sink)
            .await
        {
            Ok(answer) => Ok(Some(answer)),
            Err(error) if is_answer_failure(&error) && lines.len() >= 2 => {
                log::warn!(target: "enhance", "one pass failed ({:?}); noting in halves", error.kind());
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// [`Self::ask`], asked once more when the answer could not be used:
    /// the model samples afresh, and a loop or a cut-off rarely repeats.
    async fn ask_again(
        &self,
        prompt: Prompt,
        target: &Target,
        sink: Option<&dyn AnswerSink>,
    ) -> Result<EnhancedDoc, LlmError> {
        match self.ask(prompt.clone(), target, sink).await {
            Err(error) if is_answer_failure(&error) => {
                log::info!(target: "enhance", "notes answer unusable ({:?}); asking again", error.kind());
                self.ask(prompt, target, sink).await
            }
            other => other,
        }
    }

    /// One structured call, streamed to `sink` when there is one.
    async fn ask(
        &self,
        prompt: Prompt,
        target: &Target,
        sink: Option<&dyn AnswerSink>,
    ) -> Result<EnhancedDoc, LlmError> {
        let prompt = prompt.on(target.clone());
        match sink {
            Some(sink) => {
                self.gateway
                    .structured_streamed::<EnhancedDoc>(Route::NotesEnhance, prompt, sink)
                    .await
            }
            None => {
                self.gateway
                    .structured::<EnhancedDoc>(Route::NotesEnhance, prompt)
                    .await
            }
        }
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
