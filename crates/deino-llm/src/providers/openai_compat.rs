//! OpenAI-compatible Chat Completions, streamed: OpenAI, Groq, OpenRouter,
//! custom endpoints, and Ollama's `/v1` API ([`Flavor`]).
//!
//! JSON answers use `response_format: json_schema`. Optional request fields
//! (`response_format`, `stream_options`, `reasoning_effort`) are not
//! understood by every server, so a 400 or 422 to a request that carried any
//! of them is retried once without them, with the schema moved into the
//! system prompt (the Handy fallback, generalising the legacy Ollama retry).

mod decode;
mod flavor;
mod response;
mod wire;

use std::sync::Arc;

use async_trait::async_trait;

use self::decode::Decoder;
pub use self::flavor::Flavor;
pub(crate) use self::flavor::OLLAMA_DEFAULT_URL;
use self::response::{ModelEntry, ModelList};
use self::wire::{ChatRequest, Extras};
use super::stream::decode_sse;
use super::{optional_key, parse_body, required_key, status};
use crate::{
    CompletionRequest, CredentialError, CredentialSource, DraftConnection, Endpoint,
    EndpointSource, EventStream, HttpRequest, HttpResponse, HttpTransport, LlmError, LlmProvider,
    Method, ModelCatalog, ModelInfo, OutputFormat,
};

/// An OpenAI-compatible adapter for one [`Flavor`].
pub struct OpenAiCompatProvider {
    flavor: Flavor,
    http: Arc<dyn HttpTransport>,
    credentials: Arc<dyn CredentialSource>,
    endpoints: Arc<dyn EndpointSource>,
}

/// Where and how to send: the resolved base URL, key and sampling.
struct Connection {
    base_url: String,
    endpoint: Endpoint,
    key: Option<crate::ApiKey>,
}

impl OpenAiCompatProvider {
    /// An adapter for `flavor` over the app's ports.
    pub fn new(
        flavor: Flavor,
        http: Arc<dyn HttpTransport>,
        credentials: Arc<dyn CredentialSource>,
        endpoints: Arc<dyn EndpointSource>,
    ) -> Self {
        Self {
            flavor,
            http,
            credentials,
            endpoints,
        }
    }

    /// The connection for a completion: hosted providers need their key.
    async fn connection(&self) -> Result<Connection, LlmError> {
        let provider = self.flavor.provider();
        let key = if provider.requires_api_key() {
            Some(required_key(self.credentials.as_ref(), provider).await?)
        } else {
            optional_key(self.credentials.as_ref(), provider).await?
        };
        self.connect(key).await
    }

    async fn connect(&self, key: Option<crate::ApiKey>) -> Result<Connection, LlmError> {
        let endpoint = self.endpoint().await?;
        let base_url = self
            .flavor
            .base_url(Some(endpoint.base_url.as_str()))
            .ok_or_else(|| LlmError::Unavailable {
                provider: self.flavor.provider(),
                reason: "set the endpoint URL in Settings".to_owned(),
            })?;
        Ok(Connection {
            base_url,
            endpoint,
            key,
        })
    }

    async fn endpoint(&self) -> Result<Endpoint, LlmError> {
        if !self.flavor.is_configurable() {
            return Ok(Endpoint::default());
        }
        let provider = self.flavor.provider();
        let endpoint = self
            .endpoints
            .endpoint(provider)
            .await
            .map_err(|source: CredentialError| LlmError::Credentials { provider, source })?;
        Ok(endpoint.unwrap_or_default())
    }

    async fn post(
        &self,
        connection: &Connection,
        body: &ChatRequest<'_>,
    ) -> Result<HttpResponse, LlmError> {
        let url = format!("{}/chat/completions", connection.base_url);
        let request = authorize(HttpRequest::json(Method::Post, url, body), connection);
        let provider = self.flavor.provider();
        self.http
            .send(request)
            .await
            .map_err(status::network(provider))
    }

    fn extras(&self, request: &CompletionRequest) -> Extras {
        Extras {
            usage_chunk: true,
            response_format: matches!(request.output, OutputFormat::Json(_)),
            reasoning_effort: self.flavor.reasoning_effort(),
        }
    }
}

fn authorize(request: HttpRequest, connection: &Connection) -> HttpRequest {
    match &connection.key {
        Some(key) => request.header("authorization", format!("Bearer {}", key.expose())),
        None => request,
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn stream(&self, request: CompletionRequest) -> Result<EventStream, LlmError> {
        let provider = self.flavor.provider();
        let connection = self.connection().await?;
        let sampling = connection.endpoint.sampling;
        let mut response = self
            .post(
                &connection,
                &ChatRequest::new(&request, self.extras(&request), sampling),
            )
            .await?;
        if matches!(response.status, 400 | 422) {
            log::warn!(target: "llm", "{provider} rejected optional request fields; retrying without them");
            let plain = ChatRequest::new(&request, Extras::default(), sampling);
            response = self.post(&connection, &plain).await?;
        }
        if !response.is_success() {
            return Err(status::error_from(provider, response).await);
        }
        Ok(decode_sse(provider, response.body, Decoder::new(provider)))
    }
}

#[async_trait]
impl ModelCatalog for OpenAiCompatProvider {
    async fn models(&self, draft: &DraftConnection) -> Result<Vec<ModelInfo>, LlmError> {
        let provider = self.flavor.provider();
        let saved = optional_key(self.credentials.as_ref(), provider).await?;
        let key = draft.api_key.clone().or(saved);
        // OpenRouter lists its models publicly; the others need the key.
        if provider.requires_api_key() && key.is_none() && self.flavor != Flavor::OpenRouter {
            let fallback = self.flavor.fallback_models().iter();
            return Ok(fallback.map(|id| ModelInfo::named(*id)).collect());
        }
        let mut connection = self.connect(key).await?;
        let typed = draft.endpoint.as_deref();
        if let Some(base_url) = typed.and_then(|url| self.flavor.base_url(Some(url))) {
            connection.base_url = base_url;
        }
        let list = self.model_list(&connection).await?;
        Ok(list
            .data
            .into_iter()
            .filter(|model| self.flavor.lists(&model.id))
            .map(model_info)
            .collect())
    }
}

impl OpenAiCompatProvider {
    async fn model_list(&self, connection: &Connection) -> Result<ModelList, LlmError> {
        let provider = self.flavor.provider();
        let url = format!("{}/models", connection.base_url);
        let request = authorize(HttpRequest::get(url), connection);
        let response = self
            .http
            .send(request)
            .await
            .map_err(status::network(provider))?;
        if !response.is_success() {
            return Err(status::error_from(provider, response).await);
        }
        let body = response.bytes().await.map_err(status::network(provider))?;
        parse_body(provider, &body)
    }
}

/// OpenRouter reports the serving provider's window, which wins.
fn model_info(model: ModelEntry) -> ModelInfo {
    let context_length = model.top_provider.and_then(|top| top.context_length);
    ModelInfo {
        display_name: model.name,
        context_length: context_length.or(model.context_length),
        ..ModelInfo::named(model.id)
    }
}

#[cfg(test)]
mod openai_compat_tests;
