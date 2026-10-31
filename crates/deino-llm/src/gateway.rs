//! The gateway: the one way Deino talks to an LLM (PARITY_PLAN §3.6, §4.3).
//!
//! Every call names a [`Route`]. The gateway resolves it to a provider and
//! model ([`Router`], or the prompt's pinned [`Target`]), builds the
//! provider-agnostic request, calls the provider adapter and meters the call
//! into the usage log. Callers choose the shape of the answer:
//!
//! - [`Gateway::stream`] and [`Gateway::complete`]: text;
//! - [`Gateway::structured`]: a typed value (`structured.rs`);
//! - [`Gateway::with_tools`]: a tool-calling loop (`tools.rs`);
//! - [`Gateway::embed`]: vectors from the local embedding model.
//!
//! Nothing here logs prompt or answer content: only routes, providers,
//! models, counts and error kinds.

mod metering;

use std::sync::Arc;

use deino_domain::Clock;

use self::metering::Meter;
use crate::context::with_context;
use crate::{
    Completion, CompletionRequest, EventStream, LOCAL_EMBEDDING_MODEL, LlmError, LocalModels,
    OutputFormat, Prompt, ProviderId, Providers, Route, Router, SystemContext, Target,
    ToolDefinition, Usage, UsageRecorder,
};

/// The answer cap when a prompt sets none: room for a long summary.
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// What the gateway is built from (wired by the app's composition root).
pub struct GatewayParts {
    /// The provider adapters.
    pub providers: Providers,
    /// Route resolution.
    pub router: Router,
    /// The local models, for embeddings.
    pub local: Arc<dyn LocalModels>,
    /// Where usage records go.
    pub usage: Arc<dyn UsageRecorder>,
    /// The time source for latency and record timestamps.
    pub clock: Arc<dyn Clock>,
}

/// The LLM gateway.
pub struct Gateway {
    providers: Providers,
    router: Router,
    local: Arc<dyn LocalModels>,
    usage: Arc<dyn UsageRecorder>,
    clock: Arc<dyn Clock>,
    context: Option<Arc<dyn SystemContext>>,
}

/// The non-conversation parts of a request: the answer's shape and tools.
#[derive(Clone, Debug, Default)]
pub(crate) struct Shape {
    pub output: OutputFormat,
    pub tools: Vec<ToolDefinition>,
}

impl Gateway {
    /// A gateway over its parts.
    pub fn new(parts: GatewayParts) -> Self {
        Self {
            providers: parts.providers,
            router: parts.router,
            local: parts.local,
            usage: parts.usage,
            clock: parts.clock,
            context: None,
        }
    }

    /// Puts `context` in front of every prompt's system prompt (NOT-05: the
    /// user's profile), so every text route writes for this user.
    #[must_use]
    pub fn with_system_context(mut self, context: Arc<dyn SystemContext>) -> Self {
        self.context = Some(context);
        self
    }

    /// Route resolution, for Settings → AI.
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Streams a text answer for `route`.
    pub async fn stream(&self, route: Route, prompt: Prompt) -> Result<EventStream, LlmError> {
        self.stream_shaped(route, prompt, Shape::default()).await
    }

    /// A whole text answer for `route`.
    pub async fn complete(&self, route: Route, prompt: Prompt) -> Result<Completion, LlmError> {
        self.complete_shaped(route, prompt, Shape::default()).await
    }

    /// One vector per input from the local embedding model (`embed` is
    /// always local, §8 D3).
    pub async fn embed(&self, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        let target = Target {
            provider: ProviderId::BuiltIn,
            model: LOCAL_EMBEDDING_MODEL.to_owned(),
            effort: None,
        };
        let meter = self.meter(Route::Embed, &target);
        match self.local.embed(&target.model, inputs).await {
            Ok(vectors) => {
                meter.succeed(Usage::default());
                Ok(vectors)
            }
            Err(error) => {
                meter.fail(&error);
                Err(error)
            }
        }
    }

    /// The context window of `target`'s model, when its provider can tell.
    pub async fn context_window(&self, target: &Target) -> Option<u32> {
        let provider = self.providers.get(target.provider).ok()?;
        provider.context_window(&target.model).await
    }

    /// What serves `prompt` on `route`: its pinned target, or the route's.
    pub async fn target(&self, route: Route, prompt: &Prompt) -> Result<Target, LlmError> {
        match &prompt.target {
            Some(target) => Ok(target.clone()),
            None => Ok(self.router.resolve(route).await?.target),
        }
    }

    pub(crate) async fn complete_shaped(
        &self,
        route: Route,
        prompt: Prompt,
        shape: Shape,
    ) -> Result<Completion, LlmError> {
        Completion::collect(self.stream_shaped(route, prompt, shape).await?).await
    }

    pub(crate) async fn stream_shaped(
        &self,
        route: Route,
        prompt: Prompt,
        shape: Shape,
    ) -> Result<EventStream, LlmError> {
        let target = self.target(route, &prompt).await?;
        let effort = target.effort.map_or("", |effort| effort.as_str());
        log::info!(target: "llm", "{route}: calling {} {} {effort}", target.provider, target.model);
        let meter = self.meter(route, &target);
        let context = match &self.context {
            Some(context) => context.system_context().await,
            None => None,
        };
        let request = request(route, &target, with_context(prompt, context), shape);
        let started = match self.providers.get(target.provider) {
            Ok(provider) => provider.stream(request).await,
            Err(error) => Err(error),
        };
        match started {
            Ok(stream) => Ok(meter.wrap(stream)),
            Err(error) => {
                log::warn!(target: "llm", "{route}: {} failed: {error}", target.provider);
                meter.fail(&error);
                Err(error)
            }
        }
    }

    fn meter(&self, route: Route, target: &Target) -> Meter {
        Meter::start(self.usage.clone(), self.clock.clone(), route, target)
    }
}

fn request(route: Route, target: &Target, prompt: Prompt, shape: Shape) -> CompletionRequest {
    CompletionRequest {
        route,
        model: target.model.clone(),
        system: prompt.system,
        messages: prompt.messages,
        tools: shape.tools,
        output: shape.output,
        max_tokens: prompt.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        temperature: prompt.temperature,
        effort: target.effort,
    }
}

#[cfg(test)]
mod gateway_tests;
