//! The [`ActionItemModel`] port and its gateway adapter: the one LLM call
//! of extraction, on route `extract.action_items` (PARITY_PLAN §4.3).

use std::sync::Arc;

use async_trait::async_trait;
use deino_llm::extraction::ActionItemsAnswer;
use deino_llm::prompts::{ActionItemsSystem, ActionItemsUser, PromptText};
use deino_llm::{Gateway, LlmError, Prompt, ProviderId, Route};

/// Room for a long meeting's items: about 25 items of 150 tokens each.
const EXTRACTION_MAX_TOKENS: u32 = 4_096;

/// What the prompt carries, already rendered as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractionPrompt {
    /// The title, then the local date with its weekday and time zone.
    pub meeting: String,
    /// One line per attendee.
    pub attendees: String,
    /// The user's notes, if they wrote any.
    pub notes: Option<String>,
    /// The numbered transcript.
    pub transcript: String,
}

impl ExtractionPrompt {
    /// The gateway prompt: system instructions and the user message.
    pub fn prompt(&self) -> Prompt {
        let user = ActionItemsUser {
            meeting: &self.meeting,
            attendees: &self.attendees,
            notes: self.notes.as_deref(),
            transcript: &self.transcript,
        };
        Prompt::new(user.text())
            .system(ActionItemsSystem.text())
            .max_tokens(EXTRACTION_MAX_TOKENS)
    }
}

/// Reads a meeting's commitments.
#[async_trait]
pub trait ActionItemModel: Send + Sync {
    /// The model's answer for `prompt`, unvalidated.
    async fn extract(&self, prompt: &ExtractionPrompt) -> Result<ActionItemsAnswer, LlmError>;
}

/// No model: for an app whose gateway could not be wired. Every extraction
/// fails as unavailable, and the stage waits for the user.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoActionItemModel;

#[async_trait]
impl ActionItemModel for NoActionItemModel {
    async fn extract(&self, _prompt: &ExtractionPrompt) -> Result<ActionItemsAnswer, LlmError> {
        Err(LlmError::Unavailable {
            provider: ProviderId::BuiltIn,
            reason: "the AI gateway is not running".into(),
        })
    }
}

/// [`ActionItemModel`] over the LLM gateway.
pub struct GatewayActionItemModel {
    gateway: Arc<Gateway>,
}

impl GatewayActionItemModel {
    /// A model over `gateway`.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl ActionItemModel for GatewayActionItemModel {
    async fn extract(&self, prompt: &ExtractionPrompt) -> Result<ActionItemsAnswer, LlmError> {
        self.gateway
            .structured(Route::ExtractActionItems, prompt.prompt())
            .await
    }
}
