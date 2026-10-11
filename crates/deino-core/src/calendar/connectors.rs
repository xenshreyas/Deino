//! The direct connectors' side of [`CalendarService`]: connect (the
//! provider's sign-in in the browser, then a sync), disconnect (forget the
//! sign-in and the provider's synced calendars), and their status.

use std::collections::BTreeSet;
use std::sync::Arc;

use deino_domain::CalendarProvider;

use super::service::SyncState;
use super::{
    CalendarAccess, CalendarError, CalendarService, CalendarSource, CalendarStatus, ConnectorStatus,
};
use crate::settings::CalendarSettings;

impl CalendarService {
    /// Connects `provider`'s account: opens its sign-in page in the browser,
    /// waits for the user, then syncs it.
    pub async fn connect(
        &self,
        provider: CalendarProvider,
    ) -> Result<CalendarStatus, CalendarError> {
        let connector = self.connector(provider)?;
        connector.request_access().await?;
        self.sync_now().await;
        Ok(self.announce().await)
    }

    /// Disconnects `provider`'s account: forgets its sign-in and deletes its
    /// calendars and events (meetings keep their series, not the link).
    pub async fn disconnect(
        &self,
        provider: CalendarProvider,
    ) -> Result<CalendarStatus, CalendarError> {
        let connector = self.connector(provider)?;
        let running = self.running.lock().await;
        connector.disconnect().await?;
        for account in self.store.accounts.accounts().await? {
            if account.provider == provider {
                self.store.accounts.remove_account(&account.id).await?;
            }
        }
        drop(running);
        {
            let mut state = self.state();
            state.synced_at.remove(&provider);
            state.errors.remove(&provider);
        }
        self.refresh_nearby().await;
        Ok(self.announce().await)
    }

    /// The providers whose events Deino syncs and shows: macOS Calendar
    /// while its switch is on, each connector while connected.
    pub(super) async fn active_providers(&self) -> BTreeSet<CalendarProvider> {
        let mut active = BTreeSet::new();
        if self.settings.get().await.calendar.sync_enabled {
            active.insert(self.source.provider());
        }
        for connector in &self.connectors {
            if connector.access().await == CalendarAccess::Granted {
                active.insert(connector.provider());
            }
        }
        active
    }

    /// Each connector's setup, connection and last sync.
    pub(super) async fn connector_statuses(&self, state: &SyncState) -> Vec<ConnectorStatus> {
        let settings = self.settings.get().await.calendar;
        let mut statuses = Vec::with_capacity(self.connectors.len());
        for connector in &self.connectors {
            let provider = connector.provider();
            statuses.push(ConnectorStatus {
                provider,
                configured: configured(&settings, provider),
                connected: connector.access().await == CalendarAccess::Granted,
                last_synced_at: state.synced_at.get(&provider).copied(),
                last_error: state.errors.get(&provider).cloned(),
            });
        }
        statuses
    }

    fn connector(
        &self,
        provider: CalendarProvider,
    ) -> Result<&Arc<dyn CalendarSource>, CalendarError> {
        self.connectors
            .iter()
            .find(|connector| connector.provider() == provider)
            .ok_or(CalendarError::NoConnector(provider))
    }
}

/// True when the user entered a client ID for `provider`'s connector.
fn configured(settings: &CalendarSettings, provider: CalendarProvider) -> bool {
    let connector = match provider {
        CalendarProvider::Google => &settings.google,
        CalendarProvider::Microsoft => &settings.microsoft,
        CalendarProvider::Apple => return true,
    };
    connector.client_id().is_some()
}
