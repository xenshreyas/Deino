//! Commands of the `network` plugin: parse input, call one method of
//! `NetworkControls`, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_domain::{AppError, ServiceId};
use deino_net::{EgressEntry, NetworkControls, ServiceStatus};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![services, set_service_enabled, egress_log]
}

/// Every outside service, with its hosts and whether it may send now.
#[tauri::command]
#[specta::specta]
async fn services(
    controls: State<'_, Arc<NetworkControls>>,
) -> Result<Vec<ServiceStatus>, AppError> {
    Ok(controls.services().await)
}

/// Switches `service` on or off and returns its new status.
#[tauri::command]
#[specta::specta]
async fn set_service_enabled(
    controls: State<'_, Arc<NetworkControls>>,
    service: ServiceId,
    enabled: bool,
) -> Result<ServiceStatus, AppError> {
    controls
        .set_service_enabled(service, enabled)
        .await
        .into_ipc()
}

/// The newest `limit` outgoing requests (at most 500), newest first.
#[tauri::command]
#[specta::specta]
async fn egress_log(
    controls: State<'_, Arc<NetworkControls>>,
    limit: u32,
) -> Result<Vec<EgressEntry>, AppError> {
    controls.egress_log(limit).await.into_ipc()
}
