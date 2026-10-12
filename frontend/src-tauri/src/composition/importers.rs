//! Wiring for imports from other note-takers (CAP-12): `ImportSources` over
//! the network, the Keychain and the network switches, and `ImportJobs`
//! writing through the app's tags service and scheduling the search index
//! through the pipeline, so imported meetings are searchable and chat-able.

use std::sync::Arc;

use deino_core::importers::{ImportJobs, ImportRunner};
use deino_core::tags::TagService;
use deino_importers::ImportSources;
use deino_net::{Net, ServiceSwitch};
use deino_secrets::SecretStore;
use tauri::{AppHandle, Manager};

use super::attach::{Context, Phase};
use super::meetings::PipelineWork;
use super::network::SettingsServicePolicy;
use crate::features::importers::TauriImportEvents;

super::attach::attach!("importers", 95, Phase::Background(attach_importers));

/// Manages `Arc<ImportSources>` and `Arc<ImportJobs>`; without the network,
/// the Keychain or the tags service (a failed startup), imports stay off.
fn attach_importers(cx: &Context<'_>) {
    let Some(sources) = sources(cx.app) else {
        return log::error!(target: "importers", "the network or Keychain is missing; imports are off");
    };
    let Some(tags) = cx.app.try_state::<Arc<TagService>>() else {
        return log::error!(target: "importers", "tags are not wired; imports are off");
    };
    let runner = ImportRunner::sqlite(
        cx.database,
        Arc::clone(&tags),
        Arc::new(PipelineWork(cx.app.clone())),
        Arc::clone(cx.clock),
    );
    let events = Arc::new(TauriImportEvents::new(cx.app.clone()));
    cx.app.manage(Arc::new(ImportJobs::new(runner, events)));
    cx.app.manage(Arc::new(sources));
}

fn sources(app: &AppHandle) -> Option<ImportSources> {
    let net = app.try_state::<Net>()?;
    let secrets = app.try_state::<Arc<dyn SecretStore>>()?;
    let policy = app.try_state::<Arc<SettingsServicePolicy>>()?;
    let switch: Arc<dyn ServiceSwitch> = policy.inner().clone();
    Some(ImportSources::new(
        net.inner().clone(),
        Arc::clone(&secrets),
        switch,
    ))
}
