//! Anthropic (Claude) over the Messages API, streamed.
//!
//! Tool use is native, with `tool_choice` left to the model. JSON answers
//! are structured outputs (`output_config.format`) under the answer's schema,
//! rewritten to the subset the API accepts ([`schema`]); the answer arrives as
//! text and the gateway still parses it. `temperature` is never sent. Thinking
//! is not configured: Sonnet and Haiku 5.5 think adaptively by default (and
//! refuse `thinking: disabled`), so JSON answers get a larger `max_tokens`
//! floor and the decoder drops whatever reasoning comes back. Effort is left
//! at each model's default, since older models refuse the parameter.

mod decode;
pub mod schema;
mod wire;

use std::sync::Arc;

use async_trait::async_trait;

use self::decode::Decoder;
use self::wire::{MessagesRequest, ModelList};
use super::stream::decode_sse;
use super::{optional_key, required_key, status};
use crate::{
    ApiKey, CLAUDE_FAST_MODEL, CLAUDE_SMART_MODEL, CompletionRequest, CredentialSource,
    DraftConnection, EventStream, HttpRequest, HttpTransport, LlmError, LlmProvider, Method,
    ModelCatalog, ModelInfo, OutputFormat, ProviderId,
};

const PROVIDER: ProviderId = ProviderId::Anthropic;
const MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const MODELS_URL: &str = "https://api.anthropic.com/v1/models?limit=1000";
/// The Messages API version Deino speaks.
const API_VERSION: &str = "2023-06-01";

/// The Anthropic adapter.
pub struct AnthropicProvider {
    http: Arc<dyn HttpTransport>,
    credentials: Arc<dyn CredentialSource>,
}

impl AnthropicProvider {
    /// An adapter that reaches Anthropic through `http` with the key from `credentials`.
    pub fn new(http: Arc<dyn HttpTransport>, credentials: Arc<dyn CredentialSource>) -> Self {
        Self { http, credentials }
    }

    async fn authorized(&self, request: HttpRequest) -> Result<HttpRequest, LlmError> {
        let key = required_key(self.credentials.as_ref(), PROVIDER).await?;
        Ok(with_key(request, &key))
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn stream(&self, request: CompletionRequest) -> Result<EventStream, LlmError> {
        let json_answer = matches!(request.output, OutputFormat::Json(_));
        let body = MessagesRequest::new(&request);
        let http = self
            .authorized(HttpRequest::json(Method::Post, MESSAGES_URL, &body))
            .await?;
        let response = self
            .http
            .send(http)
            .await
            .map_err(status::network(PROVIDER))?;
        if !response.is_success() {
            return Err(status::error_from(PROVIDER, response).await);
        }
        Ok(decode_sse(
            PROVIDER,
            response.body,
            Decoder::new(json_answer),
        ))
    }
}

#[async_trait]
impl ModelCatalog for AnthropicProvider {
    async fn models(&self, draft: &DraftConnection) -> Result<Vec<ModelInfo>, LlmError> {
        let saved = optional_key(self.credentials.as_ref(), PROVIDER).await?;
        let Some(key) = draft.api_key.as_ref().or(saved.as_ref()) else {
            return Ok(default_models());
        };
        let request = with_key(HttpRequest::get(MODELS_URL), key);
        let response = self
            .http
            .send(request)
            .await
            .map_err(status::network(PROVIDER))?;
        if !response.is_success() {
            return Err(status::error_from(PROVIDER, response).await);
        }
        let body = response.bytes().await.map_err(status::network(PROVIDER))?;
        let list: ModelList = super::parse_body(PROVIDER, &body)?;
        Ok(list
            .data
            .into_iter()
            .filter(|model| model.id.starts_with("claude-"))
            .map(|model| ModelInfo {
                display_name: model.display_name,
                ..ModelInfo::named(model.id)
            })
            .collect())
    }
}

fn with_key(request: HttpRequest, key: &ApiKey) -> HttpRequest {
    request
        .header("x-api-key", key.expose())
        .header("anthropic-version", API_VERSION)
}

/// The models listed before a key is set: the §4.3 defaults.
fn default_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo::named(CLAUDE_SMART_MODEL).display_name("Claude Sonnet 5.5"),
        ModelInfo::named(CLAUDE_FAST_MODEL).display_name("Claude Haiku 5.5"),
    ]
}

#[cfg(test)]
mod anthropic_tests;
