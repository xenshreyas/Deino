//! The model behind follow-ups: the [`FollowUpModel`] port and its adapter
//! over the gateway's `followup.email` route (§4.3: the smart model), with
//! structured output for both calls.

use std::sync::Arc;

use async_trait::async_trait;
use deino_llm::followup::{FollowUpDecision, FollowUpDraftAnswer};
use deino_llm::{Gateway, LlmError, Prompt, Route};

/// Decides on and drafts follow-up emails.
#[async_trait]
pub trait FollowUpModel: Send + Sync {
    /// Whether the meeting warrants a follow-up email.
    async fn decide(&self, prompt: Prompt) -> Result<FollowUpDecision, LlmError>;

    /// The draft's subject and body, unvalidated.
    async fn draft(&self, prompt: Prompt) -> Result<FollowUpDraftAnswer, LlmError>;
}

/// [`FollowUpModel`] over the LLM gateway's `followup.email` route.
pub struct GatewayFollowUpModel {
    gateway: Arc<Gateway>,
}

impl GatewayFollowUpModel {
    /// A model over `gateway`.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl FollowUpModel for GatewayFollowUpModel {
    async fn decide(&self, prompt: Prompt) -> Result<FollowUpDecision, LlmError> {
        self.gateway.structured(Route::FollowupEmail, prompt).await
    }

    async fn draft(&self, prompt: Prompt) -> Result<FollowUpDraftAnswer, LlmError> {
        self.gateway.structured(Route::FollowupEmail, prompt).await
    }
}
