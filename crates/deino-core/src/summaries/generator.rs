//! [`SummaryGenerator`]: the legacy template summary, over the gateway.
//!
//! Three stages on the `notes.enhance` route, as the legacy processor had:
//!
//! 1. for local models (built-in, Ollama) whose context window the
//!    transcript does not fit, summarise each chunk (one retry per chunk);
//! 2. combine the chunk summaries into one;
//! 3. fill the user's Markdown template from the transcript or the combined
//!    summary.
//!
//! Hosted models read the whole transcript in one pass. Every answer is
//! cleaned of reasoning and must have visible content.

use std::sync::Arc;

use deino_llm::prompts::{
    PromptText, SummaryChunkUser, SummaryCombineUser, SummaryReportSystem, SummaryReportUser,
};
use deino_llm::{Gateway, Prompt, ProviderId, Route, Target};

use super::chunking::{chunk_text, rough_token_count};
use super::markdown::{CleanedMarkdown, clean_markdown, require_visible};
use super::{GeneratedSummary, SummaryError, SummaryRequest};

/// Tokens kept free of transcript for the prompt around it.
const PROMPT_OVERHEAD_TOKENS: u32 = 300;
/// Overlap between consecutive chunks, in tokens.
const CHUNK_OVERLAP_TOKENS: usize = 100;
/// Tries per chunk before the summary fails.
const MAX_CHUNK_ATTEMPTS: usize = 2;
/// The context assumed when a local model does not report one.
const FALLBACK_CONTEXT_TOKENS: u32 = 4000;
const CHUNK_SYSTEM: &str = "You are an expert meeting summarizer.";
const COMBINE_SYSTEM: &str = "You are an expert at synthesizing meeting summaries.";

/// Generates template summaries through the gateway.
pub struct SummaryGenerator {
    gateway: Arc<Gateway>,
}

impl SummaryGenerator {
    /// A generator over the gateway.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }

    /// The summary of `request` (see the module docs). Dropping the future
    /// cancels the provider call in flight.
    pub async fn generate(
        &self,
        request: &SummaryRequest,
    ) -> Result<GeneratedSummary, SummaryError> {
        let mut reasoning_stripped = false;
        let (source, chunks) = match self.chunk_budget(&request.target).await {
            Some(budget) if rough_token_count(&request.transcript) >= budget => {
                let summaries = self
                    .summarize_chunks(request, budget, &mut reasoning_stripped)
                    .await?;
                let chunks = summaries.len();
                (
                    self.combine(request, summaries, &mut reasoning_stripped)
                        .await?,
                    chunks,
                )
            }
            _ => (request.transcript.clone(), 1),
        };
        let report = self.report(request, &source).await?;
        Ok(GeneratedSummary {
            markdown: report.markdown,
            chunks,
            reasoning_stripped: reasoning_stripped || report.reasoning_stripped,
        })
    }

    /// The transcript size, in tokens, above which a local model gets chunks.
    async fn chunk_budget(&self, target: &Target) -> Option<usize> {
        if !matches!(target.provider, ProviderId::BuiltIn | ProviderId::Ollama) {
            return None;
        }
        let window = self
            .gateway
            .context_window(target)
            .await
            .unwrap_or(FALLBACK_CONTEXT_TOKENS);
        usize::try_from(window.saturating_sub(PROMPT_OVERHEAD_TOKENS)).ok()
    }

    async fn summarize_chunks(
        &self,
        request: &SummaryRequest,
        budget: usize,
        reasoning_stripped: &mut bool,
    ) -> Result<Vec<String>, SummaryError> {
        let chunk_tokens = budget
            .saturating_sub(usize::try_from(PROMPT_OVERHEAD_TOKENS).unwrap_or(0))
            .max(1);
        let chunks = chunk_text(&request.transcript, chunk_tokens, CHUNK_OVERLAP_TOKENS);
        let total = chunks.len();
        log::info!(target: "summaries", "summarising {total} chunks for {}", request.target.provider);
        let mut summaries = Vec::with_capacity(total);
        for (index, chunk) in chunks.iter().enumerate() {
            let cleaned = self
                .summarize_chunk(request, chunk, (index + 1, total))
                .await?;
            *reasoning_stripped |= cleaned.reasoning_stripped;
            summaries.push(cleaned.markdown);
        }
        Ok(summaries)
    }

    async fn summarize_chunk(
        &self,
        request: &SummaryRequest,
        chunk: &str,
        (number, total): (usize, usize),
    ) -> Result<CleanedMarkdown, SummaryError> {
        let user = SummaryChunkUser { chunk }.text();
        let mut attempt = 1;
        loop {
            match self
                .ask(
                    &request.target,
                    Stage {
                        name: "Summary chunk",
                        system: CHUNK_SYSTEM,
                        user: user.clone(),
                    },
                )
                .await
            {
                Ok(cleaned) => return Ok(cleaned),
                Err(error) if attempt < MAX_CHUNK_ATTEMPTS && error.is_retryable_chunk() => {
                    log::warn!(target: "summaries", "chunk {number}/{total} failed ({}); retrying", error.kind_name());
                    attempt += 1;
                }
                Err(error) => {
                    return Err(SummaryError::Chunk {
                        number,
                        total,
                        source: Box::new(error),
                    });
                }
            }
        }
    }

    async fn combine(
        &self,
        request: &SummaryRequest,
        mut summaries: Vec<String>,
        reasoning_stripped: &mut bool,
    ) -> Result<String, SummaryError> {
        if summaries.len() == 1 {
            return Ok(summaries.remove(0));
        }
        let joined = summaries.join("\n---\n");
        let user = SummaryCombineUser { summaries: &joined }.text();
        let cleaned = self
            .ask(
                &request.target,
                Stage {
                    name: "Combined summary",
                    system: COMBINE_SYSTEM,
                    user,
                },
            )
            .await?;
        *reasoning_stripped |= cleaned.reasoning_stripped;
        Ok(cleaned.markdown)
    }

    async fn report(
        &self,
        request: &SummaryRequest,
        source: &str,
    ) -> Result<CleanedMarkdown, SummaryError> {
        let system = SummaryReportSystem {
            section_instructions: &request.section_instructions,
            template_markdown: &request.template_markdown,
        }
        .text();
        let context = request
            .user_context
            .as_deref()
            .filter(|context| !context.trim().is_empty());
        let user = SummaryReportUser {
            transcript: source,
            user_context: context,
        }
        .text();
        self.ask(
            &request.target,
            Stage {
                name: "Final summary",
                system: &system,
                user,
            },
        )
        .await
    }

    /// One call on `notes.enhance` with `target`, cleaned and checked.
    async fn ask(
        &self,
        target: &Target,
        stage: Stage<'_>,
    ) -> Result<CleanedMarkdown, SummaryError> {
        let prompt = Prompt::new(stage.user)
            .system(stage.system)
            .on(target.clone());
        let completion = self.gateway.complete(Route::NotesEnhance, prompt).await?;
        let cleaned = clean_markdown(&completion.text);
        require_visible(stage.name, &cleaned)?;
        Ok(cleaned)
    }
}

/// One stage's messages.
struct Stage<'a> {
    /// For errors: "Summary chunk", "Combined summary", "Final summary".
    name: &'static str,
    system: &'a str,
    user: String,
}
