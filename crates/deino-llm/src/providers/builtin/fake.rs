//! A scripted [`LocalModels`] for tests: no sidecar, no model files.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream;

use super::{HelperEvent, HelperEventStream, HelperStop, LocalGeneration, LocalModels};
use crate::{LlmError, ModelInfo};

/// Answers generations with scripted helper lines, records what it was asked,
/// and embeds every input as a fixed vector.
#[derive(Default)]
pub struct FakeLocalModels {
    replies: Mutex<VecDeque<Vec<HelperEvent>>>,
    generations: Mutex<Vec<(String, LocalGeneration)>>,
    models: Vec<ModelInfo>,
}

impl FakeLocalModels {
    /// No replies and no models.
    pub fn new() -> Self {
        Self::default()
    }

    /// Lists `models` (all ready) with an 8k context window each.
    #[must_use]
    pub fn with_models(mut self, models: &[&str]) -> Self {
        self.models = models.iter().map(|id| ModelInfo::named(*id)).collect();
        self
    }

    /// Queues a generation that streams `text` token by word, then finishes.
    #[must_use]
    pub fn reply(self, text: &str) -> Self {
        let mut events: Vec<HelperEvent> = text
            .split_inclusive(' ')
            .map(|piece| HelperEvent::Token {
                text: piece.to_owned(),
            })
            .collect();
        let output_tokens = u32::try_from(events.len()).unwrap_or(u32::MAX);
        events.push(HelperEvent::Done {
            prompt_tokens: 10,
            output_tokens,
            stop: HelperStop::EndOfGeneration,
        });
        self.events(events)
    }

    /// Queues a generation that yields exactly `events`.
    #[must_use]
    pub fn events(self, events: Vec<HelperEvent>) -> Self {
        lock(&self.replies).push_back(events);
        self
    }

    /// Every `(model, generation)` started so far.
    pub fn generations(&self) -> Vec<(String, LocalGeneration)> {
        lock(&self.generations).clone()
    }
}

#[async_trait]
impl LocalModels for FakeLocalModels {
    async fn generate(
        &self,
        model: &str,
        generation: LocalGeneration,
    ) -> Result<HelperEventStream, LlmError> {
        lock(&self.generations).push((model.to_owned(), generation));
        let events = lock(&self.replies)
            .pop_front()
            .ok_or_else(|| LlmError::Unavailable {
                provider: crate::ProviderId::BuiltIn,
                reason: "no scripted generation".to_owned(),
            })?;
        Ok(stream::iter(events.into_iter().map(Ok)).boxed())
    }

    async fn embed(&self, _model: &str, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        Ok(inputs.iter().map(|_| vec![1.0, 0.0]).collect())
    }

    async fn models(&self) -> Vec<ModelInfo> {
        self.models.clone()
    }

    fn context_window(&self, model: &str) -> Option<u32> {
        self.models
            .iter()
            .any(|known| known.id == model)
            .then_some(8192)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
