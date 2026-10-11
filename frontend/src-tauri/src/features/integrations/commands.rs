//! Commands of the `integrations` plugin: parse input, call one method of
//! `IntegrationService`, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_domain::{AppError, Capability, ConnectionId, IntegrationProvider};
use deino_integrations::{
    AppInput, Connection, Delivery, DeliveryRequest, IntegrationService, ProviderView, Target,
    TokenInput,
};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The service, as Tauri state.
type Integrations<'a> = State<'a, Arc<IntegrationService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        providers,
        connect,
        cancel_connect,
        connect_token,
        check,
        disconnect,
        set_app,
        remove_app,
        targets,
        deliver,
        deliveries,
    ]
}

/// Every provider this build can connect, with the user's app and connections.
#[tauri::command]
#[specta::specta]
async fn providers(integrations: Integrations<'_>) -> Result<Vec<ProviderView>, AppError> {
    integrations.providers().await.into_ipc()
}

/// Connects `provider` in the browser; resolves once the user approves,
/// declines or cancels.
#[tauri::command]
#[specta::specta]
async fn connect(
    integrations: Integrations<'_>,
    provider: IntegrationProvider,
) -> Result<Connection, AppError> {
    integrations.connect(provider).await.into_ipc()
}

/// Cancels a browser sign-in to `provider`; false when none is waiting.
#[tauri::command]
#[specta::specta]
async fn cancel_connect(
    integrations: Integrations<'_>,
    provider: IntegrationProvider,
) -> Result<bool, AppError> {
    Ok(integrations.cancel_connect(provider))
}

/// Connects `provider` with a pasted token, which goes to the Keychain.
#[tauri::command]
#[specta::specta]
async fn connect_token(
    integrations: Integrations<'_>,
    provider: IntegrationProvider,
    input: TokenInput,
) -> Result<Connection, AppError> {
    integrations.connect_token(provider, input).await.into_ipc()
}

/// Asks the provider whether the connection still works, and records it.
#[tauri::command]
#[specta::specta]
async fn check(
    integrations: Integrations<'_>,
    connection: ConnectionId,
) -> Result<Connection, AppError> {
    integrations.check(connection).await.into_ipc()
}

/// Revokes (when the provider can), forgets and removes the connection.
#[tauri::command]
#[specta::specta]
async fn disconnect(
    integrations: Integrations<'_>,
    connection: ConnectionId,
) -> Result<(), AppError> {
    integrations.disconnect(connection).await.into_ipc()
}

/// Saves the user's own OAuth app for `provider`.
#[tauri::command]
#[specta::specta]
async fn set_app(
    integrations: Integrations<'_>,
    provider: IntegrationProvider,
    input: AppInput,
) -> Result<ProviderView, AppError> {
    integrations.set_app(provider, input).await.into_ipc()
}

/// Forgets the user's own OAuth app for `provider` and its secret.
#[tauri::command]
#[specta::specta]
async fn remove_app(
    integrations: Integrations<'_>,
    provider: IntegrationProvider,
) -> Result<ProviderView, AppError> {
    integrations.remove_app(provider).await.into_ipc()
}

/// Where the connection can write for `capability`.
#[tauri::command]
#[specta::specta]
async fn targets(
    integrations: Integrations<'_>,
    connection: ConnectionId,
    capability: Capability,
) -> Result<Vec<Target>, AppError> {
    integrations
        .targets(connection, capability)
        .await
        .into_ipc()
}

/// Sends one write, once per idempotency key ("Share to…").
#[tauri::command]
#[specta::specta]
async fn deliver(
    integrations: Integrations<'_>,
    request: DeliveryRequest,
) -> Result<Delivery, AppError> {
    integrations.deliver(request).await.into_ipc()
}

/// The newest `limit` deliveries (at most 200), newest first.
#[tauri::command]
#[specta::specta]
async fn deliveries(integrations: Integrations<'_>, limit: u32) -> Result<Vec<Delivery>, AppError> {
    integrations.deliveries(limit).await.into_ipc()
}
