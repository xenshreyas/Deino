//! Wiring for the LLM gateway (`crate::llm`), with the user's profile as its
//! system context (NOT-05).

use std::sync::Arc;

use deino_core::profile::ProfileContext;
use deino_core::settings::{LlmAccess, SettingsService};
use deino_db::Database;
use deino_domain::Clock;
use deino_llm::Gateway;
use deino_secrets::SecretStore;
use tauri::{AppHandle, Manager};

use crate::features;

/// Wires the LLM gateway (`crate::llm`): manages `Arc<Gateway>`,
/// `Arc<LlmAdmin>` and `Arc<SummaryGenerator>`, and starts the usage writer
/// that saves `llm_calls`. Every prompt starts with the user's profile
/// (`ProfileContext`). Needs `attach_settings` to have run.
pub(super) fn attach_llm(
    app: &AppHandle,
    database: &Database,
    settings: &Arc<SettingsService>,
    clock: Arc<dyn Clock>,
) -> Option<Arc<Gateway>> {
    let (Some(secrets), Some(net)) = (
        app.try_state::<Arc<dyn SecretStore>>(),
        app.try_state::<deino_net::Net>(),
    ) else {
        log::error!(target: "llm", "the Keychain or the network were not wired before the database; AI is unavailable");
        return None;
    };
    let access = Arc::new(LlmAccess::new(Arc::clone(settings), Arc::clone(&secrets)));
    let app_data_dir = app.path().app_data_dir().unwrap_or_default();
    let services = crate::llm::build(crate::llm::LlmPorts {
        access,
        database: database.clone(),
        app_data_dir,
        context: Arc::new(ProfileContext::new(Arc::clone(settings))),
        clock,
        net: net.inner().clone(),
    });
    // Owned for the app's lifetime: it ends when the gateway, and with it the
    // last usage channel, is dropped at exit.
    tauri::async_runtime::spawn(services.usage_writer.run());
    app.manage(services.gateway.clone());
    app.manage(services.admin);
    app.manage(services.summaries);
    let pull_events: Arc<dyn features::llm::PullEvents> =
        Arc::new(features::llm::TauriPullEvents::new(app.clone()));
    app.manage(pull_events);
    Some(services.gateway)
}
