//! Settings → AI: providers and their keys, model lists, the route table, a
//! test call per route, and Ollama model management.
//!
//! [`LlmAdmin`] is the service behind the app's `llm` plugin. It never hands
//! out a key: providers report a masked form. A route test streams the
//! model's reply to a [`TextSink`] and reports only metadata.

use std::collections::BTreeMap;
use std::sync::Arc;

use deino_domain::Clock;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use serde::Serialize;

use crate::prompts::{PromptText, RouteTestUser};
use crate::{
    CompletionEvent, CredentialSource, DraftConnection, EndpointSource, Gateway, LlmError,
    ModelCatalog, ModelInfo, OllamaProvider, Prompt, ProviderId, PullProgress, ResolvedRoute,
    Route, Target, Usage,
};

/// A route test's reply cap: one sentence.
const TEST_MAX_TOKENS: u32 = 64;

/// A provider as Settings → AI shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    /// The provider.
    pub id: ProviderId,
    /// Its display name.
    pub name: String,
    /// Whether it refuses requests without a key.
    pub requires_key: bool,
    /// The saved key, masked (`sk-a…7f3c`); `None` when no key is set.
    pub masked_key: Option<String>,
    /// The configured server address (Ollama and custom endpoints).
    pub endpoint: Option<String>,
}

/// What a route test measured. The reply itself went to the [`TextSink`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RouteTestReport {
    /// What served the test.
    pub target: Target,
    /// From request to last event, in milliseconds.
    pub latency_ms: u32,
    /// Token counts.
    pub usage: Usage,
    /// The vector length, for the `embed` route.
    pub dimensions: Option<u32>,
}

/// Receives a streamed reply, piece by piece.
pub trait TextSink: Send + Sync {
    /// The next piece of the reply.
    fn text(&self, piece: &str);
}

/// What [`LlmAdmin`] is built from.
pub struct AdminParts {
    /// The gateway (routes and calls).
    pub gateway: Arc<Gateway>,
    /// Each provider's model list.
    pub catalogs: BTreeMap<ProviderId, Arc<dyn ModelCatalog>>,
    /// Keys, for the masked display.
    pub credentials: Arc<dyn CredentialSource>,
    /// Server addresses, for display.
    pub endpoints: Arc<dyn EndpointSource>,
    /// Ollama, for pulls and deletes.
    pub ollama: Arc<OllamaProvider>,
    /// For test latencies.
    pub clock: Arc<dyn Clock>,
}

/// The Settings → AI service.
pub struct LlmAdmin {
    parts: AdminParts,
}

impl LlmAdmin {
    /// The service over its parts.
    pub fn new(parts: AdminParts) -> Self {
        Self { parts }
    }

    /// Every provider with its key status and endpoint.
    pub async fn providers(&self) -> Result<Vec<ProviderStatus>, LlmError> {
        let mut statuses = Vec::with_capacity(ProviderId::ALL.len());
        for id in ProviderId::ALL {
            statuses.push(self.provider(id).await?);
        }
        Ok(statuses)
    }

    async fn provider(&self, id: ProviderId) -> Result<ProviderStatus, LlmError> {
        let credentials = |source| LlmError::Credentials {
            provider: id,
            source,
        };
        let key = self
            .parts
            .credentials
            .api_key(id)
            .await
            .map_err(credentials)?;
        let endpoint = self
            .parts
            .endpoints
            .endpoint(id)
            .await
            .map_err(credentials)?;
        Ok(ProviderStatus {
            id,
            name: id.display_name().to_owned(),
            requires_key: id.requires_api_key(),
            masked_key: key.map(|key| key.masked()),
            endpoint: endpoint
                .map(|endpoint| endpoint.base_url)
                .filter(|url| !url.is_empty()),
        })
    }

    /// The models `provider` can serve, reached with the form's unsaved
    /// `draft` values where set.
    pub async fn models(
        &self,
        provider: ProviderId,
        draft: &DraftConnection,
    ) -> Result<Vec<ModelInfo>, LlmError> {
        let catalog = self
            .parts
            .catalogs
            .get(&provider)
            .ok_or(LlmError::Unsupported {
                provider,
                feature: "listing models",
            })?;
        catalog.models(draft).await
    }

    /// Every route and what serves it.
    pub async fn routes(&self) -> Result<Vec<ResolvedRoute>, LlmError> {
        self.parts.gateway.router().table().await
    }

    /// Pins `route` to `target`, or resets it with `None`.
    pub async fn set_route(
        &self,
        route: Route,
        target: Option<Target>,
    ) -> Result<ResolvedRoute, LlmError> {
        let resolved = self.parts.gateway.router().set(route, target).await?;
        log::info!(target: "llm", "{route} now uses {} {}", resolved.target.provider, resolved.target.model);
        Ok(resolved)
    }

    /// Sends a short test message on `route`, streaming the reply to `sink`.
    pub async fn test_route(
        &self,
        route: Route,
        sink: &dyn TextSink,
    ) -> Result<RouteTestReport, LlmError> {
        let gateway = &self.parts.gateway;
        let started = self.parts.clock.now();
        let target = gateway.router().resolve(route).await?.target;
        let (usage, dimensions) = if route == Route::Embed {
            let vectors = gateway
                .embed(vec!["Deino connection test".to_owned()])
                .await?;
            let dimensions = vectors
                .first()
                .map(|vector| u32::try_from(vector.len()).unwrap_or(u32::MAX));
            (Usage::default(), dimensions)
        } else {
            (self.stream_test(route, sink).await?, None)
        };
        let elapsed = (self.parts.clock.now() - started).num_milliseconds();
        Ok(RouteTestReport {
            target,
            latency_ms: u32::try_from(elapsed).unwrap_or(u32::MAX),
            usage,
            dimensions,
        })
    }

    async fn stream_test(&self, route: Route, sink: &dyn TextSink) -> Result<Usage, LlmError> {
        let prompt = Prompt::new(
            RouteTestUser {
                route: route.as_str(),
            }
            .text(),
        )
        .max_tokens(TEST_MAX_TOKENS);
        let mut stream = self.parts.gateway.stream(route, prompt).await?;
        let mut usage = Usage::default();
        while let Some(event) = stream.next().await {
            match event? {
                CompletionEvent::Text(piece) => sink.text(&piece),
                CompletionEvent::Usage(report) => usage = usage.merge(report),
                CompletionEvent::ToolUse(_) | CompletionEvent::Stop(_) => {}
            }
        }
        Ok(usage)
    }

    /// Downloads `model` onto the Ollama server.
    pub async fn ollama_pull(
        &self,
        model: &str,
    ) -> Result<BoxStream<'static, Result<PullProgress, LlmError>>, LlmError> {
        log::info!(target: "llm", "pulling Ollama model {model}");
        self.parts.ollama.pull(model).await
    }

    /// Deletes `model` from the Ollama server.
    pub async fn ollama_delete(&self, model: &str) -> Result<(), LlmError> {
        log::info!(target: "llm", "deleting Ollama model {model}");
        self.parts.ollama.delete(model).await
    }
}

#[cfg(test)]
mod admin_tests;
