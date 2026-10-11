//! Action item extraction (AI-05): the `extract_action_items` pipeline
//! stage's rules, behind the [`ActionItemExtractor`] port the stage calls.
//!
//! | File | Role |
//! |---|---|
//! | `context.rs` | [`ContextSources`]: the meeting's day, people, notes and numbered transcript |
//! | `llm.rs` | [`ActionItemModel`] and [`GatewayActionItemModel`] (route `extract.action_items`) |
//! | `due.rs`, `words.rs` | [`resolve_due`]: "by Friday" → a day, from the meeting's local day |
//! | `assignee.rs` | [`Roster`]: names → people (attendees, then speakers; "me"; typos) |
//! | `evidence.rs` | [`Transcript`]: `s<n>` citations checked against their quotes |
//! | `resolve.rs` | the answer → [`ActionItem`](super::ActionItem)s, without repeats of kept items |
//! | `service.rs` | [`ExtractionService`], the steps in order |
//!
//! Only clear commitments are asked for (the prompt says what is not one);
//! names, dates and citations are resolved here, deterministically, so a
//! model that slips a week or invents a line id cannot store a wrong date or
//! a broken "open at this moment" link.

mod assignee;
mod context;
mod due;
mod evidence;
mod llm;
mod resolve;
mod service;
mod words;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};
use deino_llm::LlmError;

pub use assignee::{Candidate, Roster, edit_distance};
pub use context::{ContextSources, MAX_NOTES_CHARS, MeetingContext};
pub use due::{parse_phrase, resolve_due};
pub use evidence::{Transcript, TranscriptLine};
pub use llm::{ActionItemModel, ExtractionPrompt, GatewayActionItemModel, NoActionItemModel};
pub use service::{ExtractionDeps, ExtractionService};

/// How an extraction ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtractionOutcome {
    /// The meeting has no transcript to read.
    NoTranscript,
    /// The items were saved.
    Extracted {
        /// New items saved.
        saved: usize,
        /// Untouched items from an earlier run that they replaced.
        replaced: u64,
    },
}

/// Why an extraction failed. Messages are user-safe.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractionError {
    /// No meeting has the ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// The model could not answer; the gateway's message says why.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// The database failed.
    #[error("could not load or save action items")]
    Storage(#[from] DbError),
}

impl ExtractionError {
    /// True for failures that may pass on their own (rate limits, a busy
    /// provider or database); a missing key or model needs the user.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::MeetingNotFound(_) => false,
            Self::Llm(error) => error.is_retryable(),
            Self::Storage(_) => true,
        }
    }
}

impl ErrorClass for ExtractionError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MeetingNotFound(_) => ErrorCode::NotFound,
            Self::Llm(error) => error.code(),
            Self::Storage(error) => error.code(),
        }
    }
}

/// Extracts a meeting's action items: the port the pipeline stage calls.
#[async_trait]
pub trait ActionItemExtractor: Send + Sync {
    /// Extracts `meeting`'s items, replacing the untouched ones of an
    /// earlier run.
    async fn extract(&self, meeting: &MeetingId) -> Result<ExtractionOutcome, ExtractionError>;
}
