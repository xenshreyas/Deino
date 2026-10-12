//! Commands of the `webhooks` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::webhooks::{CreatedWebhook, Delivery, Webhook, WebhookInput, WebhookService};
use deino_domain::{AppError, WebhookId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The service in Tauri state.
type Webhooks<'a> = State<'a, Arc<WebhookService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        list,
        create,
        update,
        delete,
        rotate_secret,
        send_test,
        deliveries
    ]
}

/// Every endpoint, oldest first (never their secrets).
#[tauri::command]
#[specta::specta]
async fn list(webhooks: Webhooks<'_>) -> Result<Vec<Webhook>, AppError> {
    webhooks.list().await.into_ipc()
}

/// Adds an endpoint; the answer carries its signing secret, this once.
#[tauri::command]
#[specta::specta]
async fn create(webhooks: Webhooks<'_>, input: WebhookInput) -> Result<CreatedWebhook, AppError> {
    webhooks.create(input).await.into_ipc()
}

/// Changes an endpoint; turning it on again clears an auto-disable.
#[tauri::command]
#[specta::specta]
async fn update(
    webhooks: Webhooks<'_>,
    id: WebhookId,
    input: WebhookInput,
) -> Result<Webhook, AppError> {
    webhooks.update(&id, input).await.into_ipc()
}

/// Removes an endpoint, its delivery log and its secret.
#[tauri::command]
#[specta::specta]
async fn delete(webhooks: Webhooks<'_>, id: WebhookId) -> Result<(), AppError> {
    webhooks.delete(&id).await.into_ipc()
}

/// Replaces an endpoint's signing secret; returns the new one, this once.
#[tauri::command]
#[specta::specta]
async fn rotate_secret(webhooks: Webhooks<'_>, id: WebhookId) -> Result<String, AppError> {
    webhooks.rotate_secret(&id).await.into_ipc()
}

/// Sends the newest finished meeting to an endpoint now, once.
#[tauri::command]
#[specta::specta]
async fn send_test(webhooks: Webhooks<'_>, id: WebhookId) -> Result<Delivery, AppError> {
    webhooks.send_test(&id).await.into_ipc()
}

/// An endpoint's newest deliveries.
#[tauri::command]
#[specta::specta]
async fn deliveries(webhooks: Webhooks<'_>, id: WebhookId) -> Result<Vec<Delivery>, AppError> {
    webhooks.deliveries(&id).await.into_ipc()
}
