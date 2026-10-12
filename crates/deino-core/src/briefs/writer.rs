//! The model behind briefs: the [`BriefWriter`] port and its adapter over
//! the gateway's `brief` route (§4.3: the smart model), with structured
//! output ([`BriefAnswer`]).

use std::sync::Arc;

use async_trait::async_trait;
use deino_llm::brief::BriefAnswer;
use deino_llm::{Gateway, LlmError, Prompt, Route};

/// Writes a brief's bullets from a rendered prompt.
#[async_trait]
pub trait BriefWriter: Send + Sync {
    /// The model's answer to `prompt`, unvalidated.
    async fn write(&self, prompt: Prompt) -> Result<BriefAnswer, LlmError>;
}

/// [`BriefWriter`] over the LLM gateway's `brief` route.
pub struct GatewayBriefWriter {
    gateway: Arc<Gateway>,
}

impl GatewayBriefWriter {
    /// A writer over `gateway`.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl BriefWriter for GatewayBriefWriter {
    async fn write(&self, prompt: Prompt) -> Result<BriefAnswer, LlmError> {
        self.gateway.structured(Route::Brief, prompt).await
    }
}
