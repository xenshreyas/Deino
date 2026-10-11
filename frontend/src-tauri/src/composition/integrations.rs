//! Wiring for the integrations framework (INT-00): the provider registry
//! (the sandbox in debug builds only), the SQLite connection and delivery
//! repositories, the Keychain credential vault, the user's OAuth apps in the
//! settings, `deino-net` egress under the settings policy, the system
//! browser and the clock.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::settings::{OAuthAppSettings, SettingsService};
use deino_db::Database;
use deino_domain::{Clock, IntegrationProvider};
use deino_integrations::{
    AppRegistration, ClockSleeper, CredentialVault, IntegrationError, IntegrationPorts,
    IntegrationService, NetEgress, OAuthApps, RegistryOptions, SqliteConnectionRepo,
    SqliteDeliveryRepo, registry,
};
use deino_net::Net;
use deino_secrets::KeychainSecretStore;
use tauri::{AppHandle, Manager};

use super::browser::SystemBrowser;
use super::network::SettingsServicePolicy;

/// Builds the integrations service over `database` and manages
/// `Arc<IntegrationService>`. Needs the settings and the network.
pub(crate) fn attach_integrations(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let (Some(settings), Some(net), Some(policy)) = (
        app.try_state::<Arc<SettingsService>>(),
        app.try_state::<Net>(),
        app.try_state::<Arc<SettingsServicePolicy>>(),
    ) else {
        return log::error!(target: "integrations", "settings or the network are not wired; integrations are off");
    };
    let policy = Arc::clone(&*policy);
    // The same Keychain items as the app's other secrets: service = the bundle identifier.
    let keychain = Arc::new(KeychainSecretStore::new(app.config().identifier.clone()));
    let ports = IntegrationPorts {
        registry: registry::integrations(RegistryOptions {
            sandbox: cfg!(debug_assertions),
        }),
        connections: Arc::new(SqliteConnectionRepo::new(database.clone())),
        deliveries: Arc::new(SqliteDeliveryRepo::new(database.clone())),
        vault: CredentialVault::new(keychain),
        apps: Arc::new(SettingsOAuthApps(Arc::clone(&*settings))),
        egress: Arc::new(NetEgress::new(net.inner().clone(), policy.clone(), policy)),
        browser: Arc::new(SystemBrowser),
        clock: Arc::clone(clock),
        sleeper: Arc::new(ClockSleeper::new(Arc::clone(clock))),
    };
    app.manage(Arc::new(IntegrationService::new(ports)));
}

/// [`OAuthApps`] over `Settings::integrations.apps`.
struct SettingsOAuthApps(Arc<SettingsService>);

#[async_trait]
impl OAuthApps for SettingsOAuthApps {
    async fn registration(&self, provider: IntegrationProvider) -> Option<AppRegistration> {
        let settings = self.0.get().await;
        let app = settings.integrations.apps.get(&provider)?;
        Some(AppRegistration {
            client_id: app.client_id.clone(),
            redirect_port: app.redirect_port,
        })
    }

    async fn save(
        &self,
        provider: IntegrationProvider,
        registration: Option<AppRegistration>,
    ) -> Result<(), IntegrationError> {
        self.0
            .update(|settings| {
                let apps = &mut settings.integrations.apps;
                match registration {
                    Some(app) => apps.insert(provider, settings_of(app)),
                    None => apps.remove(&provider),
                };
            })
            .await
            .map(drop)
            .map_err(|error| {
                log::warn!(target: "integrations", "could not save the {provider} app: {error}");
                IntegrationError::Settings
            })
    }
}

fn settings_of(app: AppRegistration) -> OAuthAppSettings {
    OAuthAppSettings {
        client_id: app.client_id,
        redirect_port: app.redirect_port,
    }
}

#[cfg(test)]
mod tests {
    use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents};

    use super::*;

    #[tokio::test]
    async fn registrations_round_trip_through_the_settings() {
        let store = Arc::new(FakeSettingsStore::with(None));
        let events = Arc::new(RecordedSettingsEvents::default());
        let settings = Arc::new(SettingsService::load(store, events).await.unwrap());
        let apps = SettingsOAuthApps(settings.clone());
        let app = AppRegistration {
            client_id: "123.apps".into(),
            redirect_port: Some(53_682),
        };

        apps.save(IntegrationProvider::Linear, Some(app.clone()))
            .await
            .unwrap();
        let saved = apps.registration(IntegrationProvider::Linear).await;
        apps.save(IntegrationProvider::Linear, None).await.unwrap();

        assert_eq!(saved, Some(app));
        assert!(settings.get().await.integrations.apps.is_empty());
    }
}
