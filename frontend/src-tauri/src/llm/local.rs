//! The gateway's [`LocalModels`] port over the built-in model registry and
//! the llama-helper sidecar (`summary/summary_engine`).
//!
//! A generation resolves the model's GGUF file, applies its chat template
//! and sampling, writes one `stream` (or `grammar`) request, and turns the
//! helper's lines into [`HelperEvent`]s. Dropping the stream before the
//! helper finishes stops the sidecar, so a cancelled summary stops using the
//! CPU (the legacy client did the same).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use deino_llm::{
    EmbedReply, HelperCompletion, HelperEvent, HelperEventStream, HelperRequest, HelperSampling,
    LlmError, LocalGeneration, LocalModels, ModelInfo, ProviderId,
};
use futures_util::StreamExt;
use futures_util::stream;

use crate::summary::summary_engine::client;
use crate::summary::summary_engine::models::{self, ModelDef};
use crate::summary::summary_engine::sidecar::{Exchange, SidecarManager};

const PROVIDER: ProviderId = ProviderId::BuiltIn;
/// The longest the helper may stay silent before a line (the legacy limit).
const LINE_TIMEOUT: Duration = Duration::from_mins(15);
/// Where embedding models are kept, next to `models/summary/`.
const EMBEDDING_DIR: &str = "embedding";

/// The built-in models, run by the sidecar.
pub(crate) struct SidecarLocalModels {
    app_data_dir: PathBuf,
}

impl SidecarLocalModels {
    /// Models under `<app_data_dir>/models/`.
    pub(crate) fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    async fn running(&self, model_path: PathBuf) -> Result<Arc<SidecarManager>, LlmError> {
        let manager = client::sidecar(&self.app_data_dir)
            .await
            .map_err(|error| unavailable(&error))?;
        manager
            .ensure_running(model_path)
            .await
            .map_err(|error| unavailable(&error))?;
        Ok(manager)
    }

    fn embedding_path(&self, model: &str) -> PathBuf {
        self.app_data_dir
            .join("models")
            .join(EMBEDDING_DIR)
            .join(format!("{model}.gguf"))
    }
}

fn unavailable(error: &anyhow::Error) -> LlmError {
    LlmError::Unavailable {
        provider: PROVIDER,
        reason: error.to_string(),
    }
}

#[async_trait]
impl LocalModels for SidecarLocalModels {
    async fn generate(
        &self,
        model: &str,
        generation: LocalGeneration,
    ) -> Result<HelperEventStream, LlmError> {
        let definition = models::get_model_by_name(model).ok_or_else(|| LlmError::Unavailable {
            provider: PROVIDER,
            reason: format!("'{model}' is not a built-in model"),
        })?;
        let model_path = client::cached_model_path(&self.app_data_dir, model)
            .map_err(|error| unavailable(&error))?;
        let request = completion_request(&definition, &model_path, generation)?;
        let line = serde_json::to_string(&request).map_err(|_| malformed("the request"))?;
        let manager = self.running(model_path).await?;
        let exchange = manager
            .begin_exchange(&line)
            .await
            .map_err(|error| unavailable(&error))?;
        log::info!(target: "llm", "built-in model {model} is generating");
        Ok(helper_events(exchange, manager))
    }

    async fn embed(&self, model: &str, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        let model_path = self.embedding_path(model);
        if !model_path.exists() {
            return Err(LlmError::Unavailable {
                provider: PROVIDER,
                reason: format!("the embedding model {model} is not downloaded yet"),
            });
        }
        let path = model_path.to_string_lossy().into_owned();
        let line = serde_json::to_string(&HelperRequest::Embed {
            model_path: path,
            inputs,
        })
        .map_err(|_| malformed("the request"))?;
        let manager = client::sidecar(&self.app_data_dir)
            .await
            .map_err(|error| unavailable(&error))?;
        let mut exchange = manager
            .begin_exchange(&line)
            .await
            .map_err(|error| unavailable(&error))?;
        match serde_json::from_str(&next_line(&mut exchange).await?)
            .map_err(|_| malformed("an embedding"))?
        {
            EmbedReply::Embeddings { vectors } => Ok(vectors),
            EmbedReply::Error { message } => Err(LlmError::Provider {
                provider: PROVIDER,
                status: 0,
                message,
            }),
        }
    }

    async fn models(&self) -> Vec<ModelInfo> {
        models::get_available_models()
            .into_iter()
            .map(|definition| model_info(&self.app_data_dir, definition))
            .collect()
    }

    fn context_window(&self, model: &str) -> Option<u32> {
        models::get_model_by_name(model).map(|definition| definition.context_size)
    }
}

fn model_info(app_data_dir: &Path, definition: ModelDef) -> ModelInfo {
    let ready = client::cached_model_path(app_data_dir, &definition.name).is_ok();
    ModelInfo {
        display_name: Some(definition.display_name),
        context_length: Some(definition.context_size),
        size_bytes: Some(definition.size_mb * 1024 * 1024),
        ready,
        ..ModelInfo::named(definition.name)
    }
}

/// The `stream` or `grammar` request for one generation.
fn completion_request(
    definition: &ModelDef,
    model_path: &Path,
    generation: LocalGeneration,
) -> Result<HelperRequest, LlmError> {
    let prompt =
        models::format_prompt(&definition.template, &generation.system, &generation.prompt)
            .map_err(|error| unavailable(&error))?;
    let sampling = definition.sampling.sanitize_for_llama_helper();
    let completion = HelperCompletion {
        prompt,
        max_tokens: generation.max_tokens,
        context_size: definition.context_size,
        model_path: model_path.to_string_lossy().into_owned(),
        sampling: HelperSampling {
            temperature: sampling.temperature,
            top_k: sampling.top_k,
            top_p: sampling.top_p,
            presence_penalty: sampling.presence_penalty,
            frequency_penalty: sampling.frequency_penalty,
            repeat_penalty: sampling.repeat_penalty,
            penalty_last_n: sampling.penalty_last_n,
        },
        stop_tokens: sampling.stop_tokens,
    };
    Ok(match generation.json_schema {
        Some(json_schema) => HelperRequest::Grammar {
            completion,
            json_schema,
        },
        None => HelperRequest::Stream(completion),
    })
}

fn malformed(what: &str) -> LlmError {
    LlmError::Malformed {
        provider: PROVIDER,
        detail: format!("could not encode or read {what}"),
    }
}

async fn next_line(exchange: &mut Exchange) -> Result<String, LlmError> {
    match tokio::time::timeout(LINE_TIMEOUT, exchange.next_line()).await {
        Ok(Ok(Some(line))) => Ok(line),
        Ok(Ok(None)) => Err(LlmError::Unavailable {
            provider: PROVIDER,
            reason: "the built-in model stopped unexpectedly".to_owned(),
        }),
        Ok(Err(error)) => Err(unavailable(&error)),
        Err(_) => Err(LlmError::Unavailable {
            provider: PROVIDER,
            reason: "the built-in model stopped responding".to_owned(),
        }),
    }
}

/// The helper's lines until `done` or `error`. Dropping the stream earlier
/// stops the sidecar.
fn helper_events(exchange: Exchange, manager: Arc<SidecarManager>) -> HelperEventStream {
    let guard = StopOnDrop {
        manager: Some(manager),
    };
    stream::unfold(Some((exchange, guard)), |state| async move {
        let (mut exchange, mut guard) = state?;
        let event = next_line(&mut exchange).await.and_then(|line| {
            serde_json::from_str::<HelperEvent>(&line).map_err(|_| malformed("a token"))
        });
        let finished = !matches!(event, Ok(HelperEvent::Token { .. }));
        if finished {
            guard.disarm();
            return Some((event, None));
        }
        Some((event, Some((exchange, guard))))
    })
    .boxed()
}

/// Stops the sidecar when a generation is abandoned mid-stream.
struct StopOnDrop {
    manager: Option<Arc<SidecarManager>>,
}

impl StopOnDrop {
    fn disarm(&mut self) {
        self.manager = None;
    }
}

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        let Some(manager) = self.manager.take() else {
            return;
        };
        log::info!(target: "llm", "a built-in generation was cancelled; stopping the sidecar");
        // Detached on purpose: Drop cannot wait, and the next request respawns
        // the sidecar anyway. Failure is logged.
        tauri::async_runtime::spawn(async move {
            if let Err(error) = manager.shutdown().await {
                log::warn!(target: "llm", "could not stop the sidecar: {error}");
            }
        });
    }
}
