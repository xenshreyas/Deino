//! The model behind "Ask about this meeting": the [`AskModel`] port and its
//! adapter over the gateway's `chat` route (§4.3: the smart model), which
//! streams each piece of the answer to a [`TextSink`] as it arrives.

use std::sync::Arc;

use async_trait::async_trait;
use deino_llm::{CompletionEvent, Gateway, LlmError, Prompt, ProviderId, Route, TextSink};
use futures_util::StreamExt;

/// Answers a question about one meeting.
#[async_trait]
pub trait AskModel: Send + Sync {
    /// The whole answer to `prompt`; each piece also goes to `sink`.
    async fn answer(&self, prompt: Prompt, sink: &dyn TextSink) -> Result<String, LlmError>;
}

/// [`AskModel`] over the LLM gateway's `chat` route.
pub struct GatewayAskModel {
    gateway: Arc<Gateway>,
}

impl GatewayAskModel {
    /// A model over `gateway`.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl AskModel for GatewayAskModel {
    async fn answer(&self, prompt: Prompt, sink: &dyn TextSink) -> Result<String, LlmError> {
        let mut stream = self.gateway.stream(Route::Chat, prompt).await?;
        let mut answer = String::new();
        while let Some(event) = stream.next().await {
            match event? {
                CompletionEvent::Text(piece) => {
                    sink.text(&piece);
                    answer.push_str(&piece);
                }
                CompletionEvent::Stop(_) => break,
                CompletionEvent::ToolUse(_) | CompletionEvent::Usage(_) => {}
            }
        }
        Ok(answer)
    }
}

/// No model could be wired at startup: every question fails as unavailable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoAskModel;

#[async_trait]
impl AskModel for NoAskModel {
    async fn answer(&self, _prompt: Prompt, _sink: &dyn TextSink) -> Result<String, LlmError> {
        Err(LlmError::Unavailable {
            provider: ProviderId::BuiltIn,
            reason: "no model is set up to answer questions".to_owned(),
        })
    }
}
