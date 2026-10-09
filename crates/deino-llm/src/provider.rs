//! The provider contract every adapter implements.
//!
//! One required method, [`LlmProvider::stream`] (the zed shape, reimplemented):
//! everything else has a default. Contract, checked by each adapter's wire
//! tests and by `MockProvider`:
//!
//! - the stream yields zero or more `Text`/`ToolUse`/`Usage` events and ends
//!   with exactly one `Stop`, or with an `Err` and nothing after it;
//! - `OutputFormat::Json` answers arrive as `Text` events holding the JSON;
//! - errors before the first event are returned from `stream` itself;
//! - dropping the stream cancels the request;
//! - nothing a provider logs contains prompt or answer content.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::{Completion, CompletionRequest, EventStream, LlmError, ProviderId};

/// An LLM provider adapter.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Starts the request and returns its events as they arrive.
    async fn stream(&self, request: CompletionRequest) -> Result<EventStream, LlmError>;

    /// Runs the request to completion.
    async fn complete(&self, request: CompletionRequest) -> Result<Completion, LlmError> {
        Completion::collect(self.stream(request).await?).await
    }

    /// The model's context window in tokens, when the provider can tell.
    /// Callers that chunk long transcripts use it; `None` means "large".
    async fn context_window(&self, _model: &str) -> Option<u32> {
        None
    }
}

/// The adapters the gateway can call, by provider.
#[derive(Clone, Default)]
pub struct Providers(BTreeMap<ProviderId, Arc<dyn LlmProvider>>);

impl Providers {
    /// No providers yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `provider` as the adapter for `id`.
    #[must_use]
    pub fn with(mut self, id: ProviderId, provider: Arc<dyn LlmProvider>) -> Self {
        self.0.insert(id, provider);
        self
    }

    /// The adapter for `id`.
    pub fn get(&self, id: ProviderId) -> Result<&Arc<dyn LlmProvider>, LlmError> {
        self.0.get(&id).ok_or(LlmError::Unavailable {
            provider: id,
            reason: "this build has no adapter for it".to_owned(),
        })
    }
}
