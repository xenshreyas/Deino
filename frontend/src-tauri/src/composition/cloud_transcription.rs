//! Wiring for ElevenLabs transcription after the meeting (CAP-14):
//! [`ElevenLabs`] over the scoped client of `ServiceId::ElevenLabs`, the
//! Keychain, the network switches and the settings; and the kept original
//! transcripts. Both are managed in the local phase, before the pipeline,
//! whose `final_transcription` stage asks [`cloud_choice`].

use std::sync::Arc;

use deino_cloud_transcription::{ElevenLabs, ElevenLabsParts};
use deino_core::final_transcription::{
    CloudChoice, LocalOnly, OriginalTranscriptDeps, OriginalTranscriptService,
    SqliteOriginalTranscripts,
};
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::SettingsService;
use deino_domain::ServiceId;
use deino_net::{Net, ServiceSwitch};
use deino_secrets::SecretStore;
use tauri::{AppHandle, Manager};

use super::attach::{Context, Phase};
use super::network::SettingsServicePolicy;

super::attach::attach!("cloud-transcription", 25, Phase::Local(attach_cloud));

/// Manages `Arc<ElevenLabs>` (when the network, the Keychain and the
/// settings are up) and `Arc<OriginalTranscriptService>`.
fn attach_cloud(cx: &Context<'_>) {
    if let Some(service) = elevenlabs(cx.app) {
        cx.app.manage(Arc::new(service));
    } else {
        log::error!(target: "cloud_transcription", "the network, Keychain or settings are missing; ElevenLabs transcription is off");
    }
    let segments = Arc::new(SqliteSegmentRepo::new(cx.database.clone()));
    let originals = OriginalTranscriptService::new(OriginalTranscriptDeps {
        originals: Arc::new(SqliteOriginalTranscripts::new(cx.database.clone())),
        segments: segments.clone(),
        writer: segments.clone(),
        speakers: segments,
        clock: Arc::clone(cx.clock),
    });
    cx.app.manage(Arc::new(originals));
}

fn elevenlabs(app: &AppHandle) -> Option<ElevenLabs> {
    let net = app.try_state::<Net>()?;
    let secrets = app.try_state::<Arc<dyn SecretStore>>()?;
    let policy = app.try_state::<Arc<SettingsServicePolicy>>()?;
    let settings = app.try_state::<Arc<SettingsService>>()?;
    let switch: Arc<dyn ServiceSwitch> = policy.inner().clone();
    Some(ElevenLabs::new(ElevenLabsParts {
        client: Arc::new(net.scoped(ServiceId::ElevenLabs)),
        secrets: Arc::clone(&secrets),
        switch,
        settings: settings.inner().clone(),
    }))
}

/// The stage's cloud choice: ElevenLabs once wired, else the Mac only.
pub(super) fn cloud_choice(app: &AppHandle) -> Arc<dyn CloudChoice> {
    match app.try_state::<Arc<ElevenLabs>>() {
        Some(service) => service.inner().clone(),
        None => Arc::new(LocalOnly),
    }
}
