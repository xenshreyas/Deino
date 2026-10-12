//! Wiring for outbound webhooks (DEV-01): the service behind the `webhooks`
//! plugin over the SQLite repositories, the Keychain, the redacting export
//! assembler (NOT-09) and the `webhooks` scoped client, whose hosts are the
//! saved endpoints'; the signal queue the event-port taps feed; and the
//! dispatch loop.
//!
//! - **Local** (`webhooks`): builds and manages the service and the
//!   [`WebhookTaps`] before the feature services are built, so their
//!   composition can tap their event ports ([`tap_action_items`],
//!   [`tap_insights`], `NotesAi::manage`).
//! - **Background** (`webhooks-loop`): once the pipeline runs, subscribes
//!   `meeting.ready` to the lifecycle and starts the loop.

use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use deino_core::action_items::{ActionItemEvents, SqliteActionItemRepo};
use deino_core::automations::{AssembledMeetings, MeetingSource};
use deino_core::insights::InsightGenerator;
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::redaction::{Redactor, SettingRedactor};
use deino_core::settings::SettingsService;
use deino_core::webhooks::taps::{ReadyObserver, TappedActionItemEvents, TappedInsightGenerator};
use deino_core::webhooks::{
    DispatchLoop, RepoSubjectItems, SignalQueue, SqliteDeliveryRepo, SqliteWebhookRepo,
    WebhookPorts, WebhookSecrets, WebhookService, WebhookSignal, WebhookSink,
};
use deino_db::Database;
use deino_domain::{Clock, ServiceId};
use deino_net::{HostPattern, Net};
use deino_secrets::KeychainSecretStore;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::mpsc::UnboundedReceiver;

use super::attach::{Context, Phase};
use super::export::redacting_assembler;
use super::network::{HostSource, SettingsServicePolicy};

super::attach::attach!("webhooks", 5, Phase::Local(attach_webhooks));
super::attach::attach!("webhooks-loop", 70, Phase::Background(start_webhooks));

/// The taps' sink and, until the loop starts, the queue's receiving end.
pub(crate) struct WebhookTaps {
    sink: Arc<dyn WebhookSink>,
    signals: Mutex<Option<UnboundedReceiver<WebhookSignal>>>,
}

/// The sink every tap sends to, once the webhooks are wired.
pub(crate) fn sink<R: Runtime>(app: &impl Manager<R>) -> Option<Arc<dyn WebhookSink>> {
    app.try_state::<WebhookTaps>()
        .map(|taps| Arc::clone(&taps.sink))
}

/// `inner`, also announcing `action_item.*` to the webhooks.
pub(super) fn tap_action_items(
    app: &AppHandle,
    inner: Arc<dyn ActionItemEvents>,
) -> Arc<dyn ActionItemEvents> {
    match sink(app) {
        Some(sink) => Arc::new(TappedActionItemEvents::new(inner, sink)),
        None => inner,
    }
}

/// `inner`, also announcing `insight.generated` to the webhooks.
pub(super) fn tap_insights(
    app: &AppHandle,
    inner: Arc<dyn InsightGenerator>,
) -> Arc<dyn InsightGenerator> {
    match sink(app) {
        Some(sink) => Arc::new(TappedInsightGenerator::new(inner, sink)),
        None => inner,
    }
}

/// Builds and manages the service and the taps' queue.
fn attach_webhooks(cx: &Context<'_>) {
    let (Some(settings), Some(net), Some(policy)) = (
        cx.app.try_state::<Arc<SettingsService>>(),
        cx.app.try_state::<Net>(),
        cx.app.try_state::<Arc<SettingsServicePolicy>>(),
    ) else {
        return log::error!(target: "webhooks", "settings or the network are not wired; webhooks are off");
    };
    let settings = Arc::clone(&*settings);
    let policy = Arc::clone(&*policy);
    let ports = ports(cx, &settings, (&net, &policy));
    let service = WebhookService::new(ports);
    policy.add_host_source(
        ServiceId::Webhooks,
        Arc::new(EndpointHosts(service.clone())),
    );
    let (queue, signals) = SignalQueue::new();
    cx.app.manage(WebhookTaps {
        sink: Arc::new(queue),
        signals: Mutex::new(Some(signals)),
    });
    cx.app.manage(Arc::new(service));
}

/// The service's ports over the app's adapters.
fn ports(
    cx: &Context<'_>,
    settings: &Arc<SettingsService>,
    (net, policy): (&Net, &Arc<SettingsServicePolicy>),
) -> WebhookPorts {
    let database: &Database = cx.database;
    // The same Keychain items as the app's other secrets: service = the bundle identifier.
    let keychain = Arc::new(KeychainSecretStore::new(cx.app.config().identifier.clone()));
    let meetings: Arc<dyn MeetingSource> = Arc::new(AssembledMeetings::new(
        redacting_assembler(database, Arc::clone(settings)),
        Arc::new(SqliteMeetingRepo::new(database.clone())),
    ));
    let redactor = SettingRedactor::new(settings.clone(), Arc::new(Redactor::regex_only()));
    WebhookPorts {
        webhooks: Arc::new(SqliteWebhookRepo::new(database.clone())),
        deliveries: Arc::new(SqliteDeliveryRepo::new(database.clone())),
        secrets: WebhookSecrets::new(keychain),
        meetings,
        items: Arc::new(RepoSubjectItems::new(
            Arc::new(SqliteActionItemRepo::new(database.clone())),
            Arc::new(redactor),
        )),
        client: Arc::new(net.scoped(ServiceId::Webhooks)),
        switch: Arc::clone(policy) as _,
        clock: Arc::clone(cx.clock) as Arc<dyn Clock>,
    }
}

/// Subscribes `meeting.ready` to the lifecycle and starts the loop.
fn start_webhooks(cx: &Context<'_>) {
    let (Some(service), Some(taps)) = (
        cx.app.try_state::<Arc<WebhookService>>(),
        cx.app.try_state::<WebhookTaps>(),
    ) else {
        return;
    };
    if let Some(lifecycle) = cx.app.try_state::<Arc<MeetingLifecycleService>>() {
        lifecycle.subscribe(Arc::new(ReadyObserver::new(Arc::clone(&taps.sink))));
    }
    let receiver = taps
        .signals
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    let Some(signals) = receiver else {
        return;
    };
    let (dispatch, _stop) = DispatchLoop::new(WebhookService::clone(&service), signals);
    // Lives as long as the app: pending deliveries resume at the next launch.
    tauri::async_runtime::spawn(dispatch.run());
}

/// The saved endpoints' hosts: the only ones the `webhooks` client may reach.
struct EndpointHosts(WebhookService);

#[async_trait]
impl HostSource for EndpointHosts {
    async fn hosts(&self) -> Vec<HostPattern> {
        match self.0.hosts().await {
            Ok(hosts) => hosts.iter().map(|host| HostPattern::exact(host)).collect(),
            Err(error) => {
                log::warn!(target: "webhooks", "could not read the endpoints' hosts: {error}");
                Vec::new()
            }
        }
    }
}
