//! Commands of the `llm` plugin (Settings → AI and the legacy model
//! pickers): parse input, call one `LlmAdmin` method, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_domain::AppError;
use deino_llm::{
    ApiKey, DraftConnection, LlmAdmin, ModelInfo, ProviderId, ProviderStatus, ResolvedRoute, Route,
    RouteTestReport, Target,
};
use futures_util::StreamExt;
use tauri::ipc::Channel;
use tauri::{Runtime, State};

use super::events::{ChannelSink, LlmStreamEvent, OllamaPullStatus, PullEvents};
use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        providers,
        models,
        routes,
        set_route,
        test_route,
        ollama_pull,
        ollama_delete
    ]
}

/// Every provider with its masked key and endpoint. Never a key.
#[tauri::command]
#[specta::specta]
async fn providers(llm: State<'_, Arc<LlmAdmin>>) -> Result<Vec<ProviderStatus>, AppError> {
    llm.providers().await.into_ipc()
}

/// The models `provider` can serve. `api_key` and `endpoint` are values
/// the user typed and has not saved yet; the saved ones are used without them.
#[tauri::command]
#[specta::specta]
async fn models(
    llm: State<'_, Arc<LlmAdmin>>,
    provider: ProviderId,
    api_key: Option<String>,
    endpoint: Option<String>,
) -> Result<Vec<ModelInfo>, AppError> {
    let draft = DraftConnection {
        api_key: api_key.and_then(ApiKey::new),
        endpoint: endpoint.filter(|url| !url.trim().is_empty()),
    };
    llm.models(provider, &draft).await.into_ipc()
}

/// Every task route and the provider and model that serve it.
#[tauri::command]
#[specta::specta]
async fn routes(llm: State<'_, Arc<LlmAdmin>>) -> Result<Vec<ResolvedRoute>, AppError> {
    llm.routes().await.into_ipc()
}

/// Pins `route` to `target`, or resets it to its default with `null`.
#[tauri::command]
#[specta::specta]
async fn set_route(
    llm: State<'_, Arc<LlmAdmin>>,
    route: Route,
    target: Option<Target>,
) -> Result<ResolvedRoute, AppError> {
    llm.set_route(route, target).await.into_ipc()
}

/// Sends a short test message on `route`, streaming the reply's text to
/// `on_event`, and reports what served it and how fast.
#[tauri::command]
#[specta::specta]
async fn test_route(
    llm: State<'_, Arc<LlmAdmin>>,
    route: Route,
    on_event: Channel<LlmStreamEvent>,
) -> Result<RouteTestReport, AppError> {
    llm.test_route(route, &ChannelSink(on_event))
        .await
        .into_ipc()
}

/// Downloads `model` onto the Ollama server, broadcasting `ollama-pull`
/// progress, and returns when it is ready.
#[tauri::command]
#[specta::specta]
async fn ollama_pull(
    llm: State<'_, Arc<LlmAdmin>>,
    events: State<'_, Arc<dyn PullEvents>>,
    model: String,
) -> Result<(), AppError> {
    let result = pull(&llm, events.as_ref(), &model).await;
    if let Err(error) = &result {
        events.emit(
            &model,
            OllamaPullStatus::Failed {
                message: error.to_string(),
            },
        );
    }
    result.into_ipc()
}

async fn pull(
    llm: &LlmAdmin,
    events: &dyn PullEvents,
    model: &str,
) -> Result<(), deino_llm::LlmError> {
    let mut progress = llm.ollama_pull(model).await?;
    while let Some(update) = progress.next().await {
        events.emit(model, update?.into());
    }
    Ok(())
}

/// Deletes `model` from the Ollama server.
#[tauri::command]
#[specta::specta]
async fn ollama_delete(llm: State<'_, Arc<LlmAdmin>>, model: String) -> Result<(), AppError> {
    llm.ollama_delete(&model).await.into_ipc()
}
