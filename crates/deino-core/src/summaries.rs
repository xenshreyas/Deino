//! Summaries: the legacy template summaries (chunk → combine → fill the
//! template), now generated through the LLM gateway.
//!
//! [`SummaryGenerator`] replaces the legacy `summary/processor.rs` and
//! `summary/llm_client.rs`; the app's legacy `summary/service.rs` still owns
//! the `summary_processes` rows, cancellation and the IPC until FND-08 turns
//! this into a pipeline stage and NOT-02 replaces it with enhanced notes.

mod chunking;
mod generator;
mod markdown;

use deino_domain::{ErrorClass, ErrorCode};
use deino_llm::{LlmError, Target};

pub use chunking::{chunk_text, rough_token_count};
pub use generator::SummaryGenerator;
pub use markdown::{
    CleanedMarkdown, clean_markdown, contains_reasoning_marker, first_heading, require_visible,
};

/// What to summarise, and how.
#[derive(Clone, Debug, PartialEq)]
pub struct SummaryRequest {
    /// The full transcript text.
    pub transcript: String,
    /// Context the user typed for this summary, if any.
    pub user_context: Option<String>,
    /// The template's per-section instructions.
    pub section_instructions: String,
    /// The template's Markdown skeleton.
    pub template_markdown: String,
    /// The provider and model the user chose for summaries.
    pub target: Target,
}

/// A generated summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedSummary {
    /// The filled-in template, cleaned of reasoning.
    pub markdown: String,
    /// How many transcript chunks were summarised (1 without chunking).
    pub chunks: usize,
    /// Whether model reasoning was removed from any stage's answer.
    pub reasoning_stripped: bool,
}

/// A summary failed. Messages are user-safe: stages and counts, never content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SummaryError {
    /// The provider call failed.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// A stage's answer was empty once reasoning was removed.
    #[error("{stage} returned no visible summary content after reasoning removal")]
    NoVisibleContent {
        /// The stage, for example "Final summary".
        stage: &'static str,
    },
    /// A stage's answer had an unterminated reasoning marker.
    #[error("{stage} contained an unterminated reasoning marker")]
    ReasoningMarker {
        /// The stage.
        stage: &'static str,
    },
    /// A transcript chunk failed after every attempt.
    #[error("transcript section {number} of {total} failed: {source}. Please retry.")]
    Chunk {
        /// The chunk's 1-based number.
        number: usize,
        /// How many chunks there are.
        total: usize,
        /// The last attempt's failure.
        source: Box<SummaryError>,
    },
}

impl SummaryError {
    /// Whether retrying the same chunk may help (not for cancellation, a
    /// missing key or a model that is not downloaded).
    fn is_retryable_chunk(&self) -> bool {
        match self {
            Self::Llm(error) => !matches!(
                error,
                LlmError::Cancelled
                    | LlmError::NoKey(_)
                    | LlmError::Auth(_)
                    | LlmError::Unavailable { .. }
                    | LlmError::Unsupported { .. }
            ),
            Self::NoVisibleContent { .. } | Self::ReasoningMarker { .. } => true,
            Self::Chunk { .. } => false,
        }
    }

    /// A short name of the failure, for logs.
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Llm(_) => "llm",
            Self::NoVisibleContent { .. } => "no visible content",
            Self::ReasoningMarker { .. } => "reasoning marker",
            Self::Chunk { .. } => "chunk",
        }
    }
}

impl ErrorClass for SummaryError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Llm(error) => error.code(),
            Self::NoVisibleContent { .. } | Self::ReasoningMarker { .. } => ErrorCode::Llm,
            Self::Chunk { source, .. } => source.code(),
        }
    }
}

#[cfg(test)]
mod summaries_tests;
