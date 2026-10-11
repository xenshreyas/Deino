//! Embeddings through the llama-helper sidecar (AI-01).
//!
//! The model file comes from the embedding `ModelService` (catalog entry,
//! verified download), never a guessed path. Each request runs on its own
//! task: the sidecar answers requests in order on one stdout, so a caller
//! that stops waiting (a search past its deadline) must not leave the reply
//! unread for the next request to mistake as its own. The task always reads
//! the reply; the caller may drop its handle.

use std::path::Path;
use std::sync::Arc;

use deino_core::models::ModelService;
use deino_llm::{EmbedReply, HelperRequest, LlmError, ProviderId};

use crate::summary::summary_engine::client;

const PROVIDER: ProviderId = ProviderId::BuiltIn;

/// The downloaded embedding model's file, or why there is none.
pub(super) async fn model_path(
    models: Option<&Arc<ModelService>>,
) -> Result<std::path::PathBuf, LlmError> {
    let missing = |reason: String| LlmError::Unavailable {
        provider: PROVIDER,
        reason,
    };
    let models = models.ok_or_else(|| missing("no embedding model is configured".to_owned()))?;
    let file = models
        .recommended()
        .map_err(|error| missing(error.to_string()))?;
    models
        .ready_path(&file)
        .await
        .map_err(|_| missing("the embedding model is not downloaded yet".to_owned()))
}

/// One vector per input from the embedding model at `model_path`.
pub(super) async fn embed(
    app_data_dir: &Path,
    model_path: &Path,
    inputs: Vec<String>,
) -> Result<Vec<Vec<f32>>, LlmError> {
    let line = serde_json::to_string(&HelperRequest::Embed {
        model_path: model_path.to_string_lossy().into_owned(),
        inputs,
    })
    .map_err(|_| malformed())?;
    let app_data_dir = app_data_dir.to_path_buf();
    let reply = tauri::async_runtime::spawn(async move { exchange(&app_data_dir, &line).await });
    match reply.await {
        Ok(reply) => reply,
        Err(error) => Err(unavailable(&anyhow::anyhow!(
            "the embedding task failed: {error}"
        ))),
    }
}

/// Writes the request and reads its one reply line, starting the sidecar
/// if it is not running (the process serves any model a request names).
async fn exchange(app_data_dir: &Path, line: &str) -> Result<Vec<Vec<f32>>, LlmError> {
    let manager = client::sidecar(app_data_dir)
        .await
        .map_err(|error| unavailable(&error))?;
    manager
        .ensure_started()
        .await
        .map_err(|error| unavailable(&error))?;
    let mut exchange = manager
        .begin_exchange(line)
        .await
        .map_err(|error| unavailable(&error))?;
    let reply = super::local::next_line(&mut exchange).await?;
    match serde_json::from_str(&reply).map_err(|_| malformed())? {
        EmbedReply::Embeddings { vectors } => Ok(vectors),
        EmbedReply::Error { message } => Err(LlmError::Provider {
            provider: PROVIDER,
            status: 0,
            message,
        }),
    }
}

fn unavailable(error: &anyhow::Error) -> LlmError {
    LlmError::Unavailable {
        provider: PROVIDER,
        reason: error.to_string(),
    }
}

fn malformed() -> LlmError {
    LlmError::Malformed {
        provider: PROVIDER,
        detail: "could not encode or read an embedding".to_owned(),
    }
}
