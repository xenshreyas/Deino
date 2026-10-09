//! Ollama: chat through its OpenAI-compatible `/v1` API, models, context
//! windows, pulls and deletes through its native API.
//!
//! The server is the user's (`http://localhost:11434` unless configured), so
//! a key is optional. Pull progress streams as [`PullProgress`].

mod native;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::stream::BoxStream;
use futures_util::{StreamExt, future};
use serde::Serialize;

use self::native::{ModelRequest, PullLine, Show, Tags};
use super::openai_compat::{Flavor, OLLAMA_DEFAULT_URL, OpenAiCompatProvider};
use super::{optional_key, parse_body, status};
use crate::{
    CompletionRequest, CredentialSource, DraftConnection, EndpointSource, EventStream, HttpRequest,
    HttpResponse, HttpTransport, LlmError, LlmProvider, Method, ModelCatalog, ModelInfo,
    ProviderId,
};

const PROVIDER: ProviderId = ProviderId::Ollama;
/// Pulls download gigabytes; the legacy limit.
const PULL_TIMEOUT: Duration = Duration::from_mins(10);

/// How a model pull is going.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PullProgress {
    /// Downloading; `percent` of the current layer is done.
    Downloading {
        /// 0 to 100.
        percent: u8,
    },
    /// The model is ready.
    Done,
}

/// The Ollama adapter.
pub struct OllamaProvider {
    chat: OpenAiCompatProvider,
    http: Arc<dyn HttpTransport>,
    credentials: Arc<dyn CredentialSource>,
    endpoints: Arc<dyn EndpointSource>,
}

impl OllamaProvider {
    /// An adapter over the app's ports.
    pub fn new(
        http: Arc<dyn HttpTransport>,
        credentials: Arc<dyn CredentialSource>,
        endpoints: Arc<dyn EndpointSource>,
    ) -> Self {
        let chat = OpenAiCompatProvider::new(
            Flavor::Ollama,
            http.clone(),
            credentials.clone(),
            endpoints.clone(),
        );
        Self {
            chat,
            http,
            credentials,
            endpoints,
        }
    }

    /// Downloads `model` onto the server, reporting progress as it goes.
    pub async fn pull(
        &self,
        model: &str,
    ) -> Result<BoxStream<'static, Result<PullProgress, LlmError>>, LlmError> {
        let body = ModelRequest {
            model,
            verbose: None,
            stream: Some(true),
        };
        let request = self
            .native(Method::Post, "pull", &body)
            .await?
            .timeout(PULL_TIMEOUT);
        let response = self.send(request).await?;
        Ok(native::lines(response.body)
            .map(|line| line.map_err(status::network(PROVIDER)).and_then(|line| progress(&line)))
            .filter_map(|progress| future::ready(progress.transpose()))
            // The first error ends the pull, as the provider contract requires.
            .scan(false, |failed, progress| {
                let next = (!*failed).then_some(progress);
                *failed = next.as_ref().is_some_and(Result::is_err);
                future::ready(next)
            })
            .boxed())
    }

    /// Deletes `model` from the server.
    pub async fn delete(&self, model: &str) -> Result<(), LlmError> {
        let body = ModelRequest {
            model,
            verbose: None,
            stream: None,
        };
        let request = self.native(Method::Delete, "delete", &body).await?;
        self.send(request).await.map(drop)
    }

    async fn base_url(&self) -> Result<String, LlmError> {
        let endpoint =
            self.endpoints
                .endpoint(PROVIDER)
                .await
                .map_err(|source| LlmError::Credentials {
                    provider: PROVIDER,
                    source,
                })?;
        let configured =
            endpoint.map(|endpoint| endpoint.base_url.trim().trim_end_matches('/').to_owned());
        Ok(configured
            .filter(|url| !url.is_empty())
            .unwrap_or_else(|| OLLAMA_DEFAULT_URL.to_owned()))
    }

    async fn native(
        &self,
        method: Method,
        path: &str,
        body: &impl Serialize,
    ) -> Result<HttpRequest, LlmError> {
        let url = format!("{}/api/{path}", self.base_url().await?);
        let request = HttpRequest::json(method, url, body).timeout(crate::METADATA_TIMEOUT);
        self.authorize(request).await
    }

    async fn authorize(&self, request: HttpRequest) -> Result<HttpRequest, LlmError> {
        Ok(
            match optional_key(self.credentials.as_ref(), PROVIDER).await? {
                Some(key) => request.header("authorization", format!("Bearer {}", key.expose())),
                None => request,
            },
        )
    }

    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, LlmError> {
        let response = self.http.send(request).await.map_err(unreachable_server)?;
        if response.is_success() {
            Ok(response)
        } else {
            Err(status::error_from(PROVIDER, response).await)
        }
    }

    async fn show(&self, model: &str) -> Result<Show, LlmError> {
        let body = ModelRequest {
            model,
            verbose: Some(true),
            stream: None,
        };
        let request = self.native(Method::Post, "show", &body).await?;
        let response = self.send(request).await?;
        let body = response.bytes().await.map_err(status::network(PROVIDER))?;
        parse_body(PROVIDER, &body)
    }
}

/// A refused connection means the server is not running: say so.
fn unreachable_server(source: crate::HttpError) -> LlmError {
    match source {
        crate::HttpError::Connect(_) => LlmError::Unavailable {
            provider: PROVIDER,
            reason: "the Ollama server is not running; install or start Ollama".to_owned(),
        },
        source => LlmError::Network {
            provider: PROVIDER,
            source,
        },
    }
}

fn progress(line: &str) -> Result<Option<PullProgress>, LlmError> {
    let line: PullLine = parse_body(PROVIDER, line.as_bytes())?;
    if let Some(message) = line.error {
        return Err(status::classify(PROVIDER, 0, None, message));
    }
    if line.status.as_deref() == Some("success") {
        return Ok(Some(PullProgress::Done));
    }
    Ok(match (line.completed, line.total) {
        (Some(completed), Some(total)) if total > 0 => {
            let percent = u8::try_from(completed.min(total) * 100 / total).unwrap_or(100);
            Some(PullProgress::Downloading { percent })
        }
        _ => None,
    })
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn stream(&self, request: CompletionRequest) -> Result<EventStream, LlmError> {
        self.chat.stream(request).await
    }

    async fn context_window(&self, model: &str) -> Option<u32> {
        match self.show(model).await {
            Ok(show) => show.context_window(model),
            Err(error) => {
                log::warn!(target: "llm", "could not read Ollama model details ({:?}); guessing", error.kind());
                native::family_default(model)
            }
        }
    }
}

#[async_trait]
impl ModelCatalog for OllamaProvider {
    async fn models(&self, draft: &DraftConnection) -> Result<Vec<ModelInfo>, LlmError> {
        let typed = draft
            .endpoint
            .as_deref()
            .map(|url| url.trim().trim_end_matches('/'));
        let base_url = match typed.filter(|url| !url.is_empty()) {
            Some(url) => url.to_owned(),
            None => self.base_url().await?,
        };
        let request = self
            .authorize(HttpRequest::get(format!("{base_url}/api/tags")))
            .await?;
        let response = self.send(request).await?;
        let body = response.bytes().await.map_err(status::network(PROVIDER))?;
        let tags: Tags = parse_body(PROVIDER, &body)?;
        Ok(tags
            .models
            .into_iter()
            .map(|tag| ModelInfo {
                size_bytes: tag.size,
                modified_at: tag.modified_at,
                ..ModelInfo::named(tag.name)
            })
            .collect())
    }
}

#[cfg(test)]
mod ollama_tests;
