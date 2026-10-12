//! Network wiring (FND-12): the one [`Net`], its policy over the settings,
//! and the egress log in the database.
//!
//! - [`attach_network`] runs right after the settings: it builds the root
//!   client, the [`SettingsServicePolicy`] and the in-memory egress log, and
//!   manages [`Net`]. Consumers take a scoped client from it, never the root.
//! - [`attach_network_log`] runs once `deino.sqlite` is open: entries start
//!   flowing into the `egress_log` table (with everything recorded before),
//!   and the Settings → Privacy → Network page gets its [`NetworkControls`].

use std::sync::{Arc, PoisonError, RwLock};

use async_trait::async_trait;
use deino_core::settings::{AiSettings, SettingsService};
use deino_db::Database;
use deino_domain::{ServiceId, SystemClock};
use deino_net::{
    EgressLog, Grant, HostPattern, Net, NetClient, NetError, NetParts, NetworkControls,
    ReqwestNetClient, ServicePolicy, ServiceSwitch, SqliteEgressLog, SwitchError, host_of,
    registry,
};
use tauri::{AppHandle, Manager, Runtime};

use super::attach::Phase;

/// Builds the network over the managed `Arc<SettingsService>` and manages
/// [`Net`] and the policy. Runs after [`super::attach_settings`].
pub(crate) fn attach_network(app: &AppHandle) -> Result<(), NetError> {
    let settings = Arc::clone(&*app.state::<Arc<SettingsService>>());
    let policy = Arc::new(SettingsServicePolicy::new(settings));
    let net = Net::new(NetParts {
        client: Arc::new(root_client()?),
        policy: policy.clone(),
        egress: Arc::new(EgressLog::default()),
        clock: Arc::new(SystemClock),
    });
    app.manage(net);
    app.manage(policy);
    Ok(())
}

/// Identifies the app to the services it calls.
const USER_AGENT: &str = concat!("Deino/", env!("CARGO_PKG_VERSION"));

/// The root HTTP client. Only [`attach_network`] wraps it for the app;
/// tests use it directly to reach their loopback servers.
pub(crate) fn root_client() -> Result<ReqwestNetClient, NetError> {
    ReqwestNetClient::with_user_agent(USER_AGENT)
}

super::attach::attach!(
    "egress-log",
    60,
    Phase::Local(|cx| attach_network_log(cx.app, cx.database))
);

/// Persists the egress log in `database` and manages the Network page's
/// [`NetworkControls`]. Does nothing if [`attach_network`] did not run.
pub(crate) fn attach_network_log(app: &AppHandle, database: &Database) {
    let (Some(net), Some(policy)) = (
        app.try_state::<Net>(),
        app.try_state::<Arc<SettingsServicePolicy>>(),
    ) else {
        log::warn!(target: "net", "the network was not wired before the database opened");
        return;
    };
    let store = Arc::new(SqliteEgressLog::new(database.clone()));
    let persister = net.egress().start_persisting();
    // Lives as long as the app: it drains the log's queue into the table.
    tauri::async_runtime::spawn(persister.run(store.clone()));
    let policy = Arc::clone(&*policy);
    app.manage(Arc::new(NetworkControls::new(
        policy.clone(),
        policy,
        store,
    )));
}

/// The client for model downloads the user starts (Hugging Face, allowed in
/// offline mode), for the legacy model managers that only have an `AppHandle`.
pub(crate) fn model_downloads<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Arc<dyn NetClient>, String> {
    let net = app
        .try_state::<Net>()
        .ok_or("the network client is not ready yet")?;
    let client = net
        .approved_download(ServiceId::HfModels)
        .map_err(|error| error.to_string())?;
    Ok(Arc::new(client))
}

/// Hosts a feature's own data adds to a service while it names them (the
/// webhook endpoints the user saved, DEV-01).
#[async_trait]
pub(crate) trait HostSource: Send + Sync {
    /// The hosts allowed right now.
    async fn hosts(&self) -> Vec<HostPattern>;
}

/// [`ServicePolicy`] and [`ServiceSwitch`] over the typed settings: the
/// `network` section's switches and offline mode, plus the custom LLM
/// endpoints' hosts and any [`HostSource`] a feature registered. Every
/// request reads them afresh, so a change applies to the very next request.
pub(crate) struct SettingsServicePolicy {
    settings: Arc<SettingsService>,
    sources: RwLock<Vec<(ServiceId, Arc<dyn HostSource>)>>,
}

impl SettingsServicePolicy {
    /// A policy over `settings`.
    pub(crate) fn new(settings: Arc<SettingsService>) -> Self {
        Self {
            settings,
            sources: RwLock::new(Vec::new()),
        }
    }

    /// Adds `source`'s hosts to `service` (a service whose registry entry
    /// takes custom hosts).
    pub(crate) fn add_host_source(&self, service: ServiceId, source: Arc<dyn HostSource>) {
        self.sources
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .push((service, source));
    }

    /// The hosts the registered sources add to `service`.
    async fn sourced_hosts(&self, service: ServiceId) -> Vec<HostPattern> {
        let sources: Vec<Arc<dyn HostSource>> = self
            .sources
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|(owner, _)| *owner == service)
            .map(|(_, source)| Arc::clone(source))
            .collect();
        let mut hosts = Vec::new();
        for source in sources {
            hosts.extend(source.hosts().await);
        }
        hosts
    }
}

#[async_trait]
impl ServicePolicy for SettingsServicePolicy {
    async fn grant(&self, service: ServiceId) -> Grant {
        let settings = self.settings.get().await;
        let default = registry::spec(service).enabled_by_default;
        let mut custom_hosts = custom_hosts(&settings.ai, service);
        custom_hosts.extend(self.sourced_hosts(service).await);
        Grant {
            enabled: settings.network.choice(service).unwrap_or(default),
            offline_mode: settings.network.offline_mode,
            custom_hosts,
        }
    }
}

#[async_trait]
impl ServiceSwitch for SettingsServicePolicy {
    async fn set_enabled(&self, service: ServiceId, enabled: bool) -> Result<(), SwitchError> {
        self.settings
            .update(|settings| {
                settings.network.services.insert(service, enabled);
            })
            .await
            .map(drop)
            .map_err(|error| SwitchError(Box::new(error)))
    }
}

/// The host a service's configured endpoint adds, if it takes one.
fn custom_hosts(ai: &AiSettings, service: ServiceId) -> Vec<HostPattern> {
    let endpoint = match service {
        ServiceId::LlmCustom => ai
            .custom_openai
            .as_ref()
            .map(|custom| custom.endpoint.as_str()),
        ServiceId::LlmOllama => ai.ollama_endpoint.as_deref(),
        _ => None,
    };
    endpoint
        .and_then(host_of)
        .map(|host| HostPattern::exact(&host))
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use deino_core::settings::{
        CustomOpenAiSettings, FakeSettingsStore, RecordedSettingsEvents, Settings,
    };

    use super::*;

    async fn policy_over(settings: Settings) -> (SettingsServicePolicy, Arc<SettingsService>) {
        let store = Arc::new(FakeSettingsStore::with(Some(settings)));
        let events = Arc::new(RecordedSettingsEvents::default());
        let service = Arc::new(SettingsService::load(store, events).await.unwrap());
        (SettingsServicePolicy::new(service.clone()), service)
    }

    #[tokio::test]
    async fn services_follow_their_defaults_until_the_user_switches_them() {
        let (policy, settings) = policy_over(Settings::default()).await;

        let before = policy.grant(ServiceId::Slack).await;
        policy.set_enabled(ServiceId::Slack, true).await.unwrap();
        let after = policy.grant(ServiceId::Slack).await;

        assert!(!before.enabled && !before.offline_mode);
        assert!(after.enabled);
        assert!(policy.grant(ServiceId::LlmAnthropic).await.enabled);
        let saved = settings.get().await.network.services;
        assert_eq!(saved.get(&ServiceId::Slack), Some(&true));
    }

    #[tokio::test]
    async fn custom_endpoints_add_their_host_and_offline_mode_is_reported() {
        let mut settings = Settings::default();
        settings.network.offline_mode = true;
        settings.ai.ollama_endpoint = Some("http://gpu-box.local:11434".into());
        settings.ai.custom_openai = Some(CustomOpenAiSettings {
            endpoint: "https://llm.example.test/v1".into(),
            model: "m".into(),
            ..CustomOpenAiSettings::default()
        });
        let (policy, _) = policy_over(settings).await;

        let ollama = policy.grant(ServiceId::LlmOllama).await;
        let custom = policy.grant(ServiceId::LlmCustom).await;

        assert!(ollama.offline_mode);
        assert_eq!(ollama.custom_hosts, [HostPattern::exact("gpu-box.local")]);
        assert_eq!(
            custom.custom_hosts,
            [HostPattern::exact("llm.example.test")]
        );
        assert!(
            policy
                .grant(ServiceId::LlmOpenAi)
                .await
                .custom_hosts
                .is_empty()
        );
    }

    struct Endpoints;

    #[async_trait]
    impl HostSource for Endpoints {
        async fn hosts(&self) -> Vec<HostPattern> {
            vec![HostPattern::exact("hooks.zapier.com")]
        }
    }

    #[tokio::test]
    async fn a_host_source_adds_its_hosts_to_its_service_only() {
        let (policy, _) = policy_over(Settings::default()).await;

        policy.add_host_source(ServiceId::Webhooks, Arc::new(Endpoints));

        let webhooks = policy.grant(ServiceId::Webhooks).await;
        assert_eq!(
            webhooks.custom_hosts,
            [HostPattern::exact("hooks.zapier.com")]
        );
        assert!(policy.grant(ServiceId::Slack).await.custom_hosts.is_empty());
    }
}
