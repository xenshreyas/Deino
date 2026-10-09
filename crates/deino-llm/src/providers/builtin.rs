//! The built-in local model: llama.cpp in the llama-helper sidecar.
//!
//! The adapter turns a request into one [`LocalGeneration`] (system prompt,
//! flattened conversation, optional JSON schema) and the helper's stdout
//! events ([`HelperEvent`], the `stream`/`grammar` protocol of
//! `llama-helper/src/protocol.rs`) back into completion events. Everything
//! model-specific (finding the GGUF, its chat template, its sampling, the
//! sidecar process) sits behind the [`LocalModels`] port, which the app
//! implements over its model registry and sidecar manager.
//!
//! JSON answers are enforced with a GBNF grammar the helper builds from the
//! schema. Tool calling is not supported by the local models.

#[cfg(any(test, feature = "test-support"))]
mod fake;
mod protocol;

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use serde::Deserialize;
use serde_json::Value;

use super::status;
use crate::{
    CompletionEvent, CompletionRequest, EventStream, LlmError, LlmProvider, Message, ModelCatalog,
    ModelInfo, OutputFormat, Part, ProviderId, Role, StopReason, Usage,
};

#[cfg(any(test, feature = "test-support"))]
pub use self::fake::FakeLocalModels;
pub use self::protocol::{EmbedReply, HelperCompletion, HelperRequest, HelperSampling};

const PROVIDER: ProviderId = ProviderId::BuiltIn;

/// One generation for the local model.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalGeneration {
    /// Instructions for the model.
    pub system: String,
    /// The conversation, flattened into one user turn.
    pub prompt: String,
    /// The most tokens to generate.
    pub max_tokens: u32,
    /// A JSON schema the answer must match (the helper's `grammar` request).
    pub json_schema: Option<Value>,
}

/// One line the helper writes while generating.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HelperEvent {
    /// The next decoded piece of text.
    Token {
        /// The piece.
        text: String,
    },
    /// Generation finished.
    Done {
        /// Prompt tokens read.
        prompt_tokens: u32,
        /// Tokens generated.
        output_tokens: u32,
        /// Why it stopped.
        stop: HelperStop,
    },
    /// Generation failed; nothing follows.
    Error {
        /// What went wrong.
        message: String,
    },
}

/// Why the helper stopped generating.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperStop {
    /// The model ended its turn.
    EndOfGeneration,
    /// `max_tokens` was reached.
    MaxTokens,
    /// One of the model's stop tokens appeared.
    StopToken,
}

/// The helper's events for one generation, ending with `Done` or `Error`.
pub type HelperEventStream = BoxStream<'static, Result<HelperEvent, LlmError>>;

/// The local models and the sidecar that runs them.
#[async_trait]
pub trait LocalModels: Send + Sync {
    /// Starts `generation` on the local model `model`. Dropping the stream
    /// stops the generation.
    async fn generate(
        &self,
        model: &str,
        generation: LocalGeneration,
    ) -> Result<HelperEventStream, LlmError>;

    /// One L2-normalised vector per input, in order, from the embedding model `model`.
    async fn embed(&self, model: &str, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError>;

    /// Every local model, downloaded or not.
    async fn models(&self) -> Vec<ModelInfo>;

    /// The context window of `model`, if it is a known model.
    fn context_window(&self, model: &str) -> Option<u32>;
}

/// The built-in adapter.
pub struct BuiltInProvider {
    local: std::sync::Arc<dyn LocalModels>,
}

impl BuiltInProvider {
    /// An adapter over the app's local models.
    pub fn new(local: std::sync::Arc<dyn LocalModels>) -> Self {
        Self { local }
    }
}

#[async_trait]
impl LlmProvider for BuiltInProvider {
    async fn stream(&self, request: CompletionRequest) -> Result<EventStream, LlmError> {
        let generation = generation(&request)?;
        let events = self.local.generate(&request.model, generation).await?;
        Ok(events
            .flat_map(|event| stream::iter(completion_events(event)))
            .boxed())
    }

    async fn context_window(&self, model: &str) -> Option<u32> {
        self.local.context_window(model)
    }
}

#[async_trait]
impl ModelCatalog for BuiltInProvider {
    async fn models(&self, _draft: &crate::DraftConnection) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(self.local.models().await)
    }
}

fn generation(request: &CompletionRequest) -> Result<LocalGeneration, LlmError> {
    let unsupported = LlmError::Unsupported {
        provider: PROVIDER,
        feature: "tool calling",
    };
    if !request.tools.is_empty() {
        return Err(unsupported);
    }
    let json_schema = match &request.output {
        OutputFormat::Json(json) => Some(json.schema.clone()),
        OutputFormat::Text => None,
    };
    Ok(LocalGeneration {
        system: request.system.clone().unwrap_or_default(),
        prompt: flatten(&request.messages).ok_or(unsupported)?,
        max_tokens: request.max_tokens,
        json_schema,
    })
}

/// One user message passes through as is; a longer conversation becomes a
/// transcript of turns. `None` when a message holds tool parts.
fn flatten(messages: &[Message]) -> Option<String> {
    let has_tool_parts = messages
        .iter()
        .flat_map(|message| &message.parts)
        .any(|part| !matches!(part, Part::Text(_)));
    if has_tool_parts {
        return None;
    }
    if let [only] = messages {
        return Some(only.text());
    }
    let turns: Vec<String> = messages
        .iter()
        .map(|message| match message.role {
            Role::User => format!("User: {}", message.text()),
            Role::Assistant => format!("Assistant: {}", message.text()),
        })
        .collect();
    Some(turns.join("\n\n"))
}

/// The completion events of one helper line (a `Done` line yields two).
fn completion_events(
    event: Result<HelperEvent, LlmError>,
) -> Vec<Result<CompletionEvent, LlmError>> {
    match event {
        Ok(HelperEvent::Token { text }) => vec![Ok(CompletionEvent::Text(text))],
        Ok(HelperEvent::Done {
            prompt_tokens,
            output_tokens,
            stop,
        }) => {
            let usage = Usage {
                input_tokens: prompt_tokens,
                output_tokens,
            };
            vec![
                Ok(CompletionEvent::Usage(usage)),
                Ok(CompletionEvent::Stop(stop_reason(stop))),
            ]
        }
        Ok(HelperEvent::Error { message }) => {
            vec![Err(status::classify(PROVIDER, 0, None, message))]
        }
        Err(error) => vec![Err(error)],
    }
}

fn stop_reason(stop: HelperStop) -> StopReason {
    match stop {
        HelperStop::EndOfGeneration => StopReason::EndTurn,
        HelperStop::MaxTokens => StopReason::MaxTokens,
        HelperStop::StopToken => StopReason::StopSequence,
    }
}

#[cfg(test)]
mod builtin_tests;
