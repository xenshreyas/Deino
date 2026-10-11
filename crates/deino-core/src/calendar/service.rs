//! [`CalendarService`]: calendar sync as the app sees it.
//!
//! This file holds the sync lifecycle: macOS Calendar's switch (`enable`
//! asks macOS for access, §3C), `refresh` and the scheduler's
//! `sync_if_enabled`, and the status Settings shows. `connectors.rs` holds
//! the direct connectors' connect and disconnect, `reads.rs` what the UI
//! lists. Syncs never overlap: one lock serialises the scheduler, wake-ups,
//! connecting and the Refresh button.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};

use chrono::{DateTime, Utc};
use deino_domain::{CalendarProvider, Clock, Email};

use super::entities::AttendeeDirectory;
use super::sync::CalendarSync;
use super::{
    CalendarAccess, CalendarAccountRepo, CalendarError, CalendarEvent, CalendarEventRepo,
    CalendarEvents, CalendarRepo, CalendarSource, CalendarStatus, CalendarSyncRepo, SourceError,
};
use crate::settings::SettingsService;

/// The calendar repositories the service reads.
#[derive(Clone)]
pub struct CalendarStore {
    pub(super) accounts: Arc<dyn CalendarAccountRepo>,
    pub(super) calendars: Arc<dyn CalendarRepo>,
    pub(super) events: Arc<dyn CalendarEventRepo>,
    pub(super) sync: Arc<dyn CalendarSyncRepo>,
}

impl CalendarStore {
    /// Every calendar port over one repository (SQLite in the app, the fake
    /// in tests).
    pub fn over<R>(repo: &Arc<R>) -> Self
    where
        R: CalendarAccountRepo + CalendarRepo + CalendarEventRepo + CalendarSyncRepo + 'static,
    {
        Self {
            accounts: repo.clone(),
            calendars: repo.clone(),
            events: repo.clone(),
            sync: repo.clone(),
        }
    }
}

/// What the service is built from.
pub struct CalendarDeps {
    /// macOS Calendar (`EventKit`), the default source.
    pub source: Arc<dyn CalendarSource>,
    /// The direct connectors (Google, Microsoft), synced while connected.
    pub connectors: Vec<Arc<dyn CalendarSource>>,
    /// The stored calendars.
    pub store: CalendarStore,
    /// Where synced attendees become people and companies.
    pub directory: AttendeeDirectory,
    /// Holds the sync switch, the hidden calendars and the client IDs.
    pub settings: Arc<SettingsService>,
    /// Where `CalendarChanged` goes.
    pub events: Arc<dyn CalendarEvents>,
    /// The time.
    pub clock: Arc<dyn Clock>,
}

/// The outcome of each source's last sync this session.
#[derive(Clone, Debug, Default)]
pub(super) struct SyncState {
    pub(super) synced_at: BTreeMap<CalendarProvider, DateTime<Utc>>,
    pub(super) errors: BTreeMap<CalendarProvider, String>,
}

/// Syncs the user's calendars and answers what the UI asks about them.
pub struct CalendarService {
    pub(super) source: Arc<dyn CalendarSource>,
    pub(super) connectors: Vec<Arc<dyn CalendarSource>>,
    pub(super) store: CalendarStore,
    pub(super) settings: Arc<SettingsService>,
    pub(super) events: Arc<dyn CalendarEvents>,
    pub(super) clock: Arc<dyn Clock>,
    /// The visible events around now, for synchronous lookups (`events_near`).
    pub(super) nearby: RwLock<Vec<CalendarEvent>>,
    pub(super) running: tokio::sync::Mutex<()>,
    sync: CalendarSync,
    state: Mutex<SyncState>,
}

impl CalendarService {
    /// The service over its ports.
    pub fn new(deps: CalendarDeps) -> Self {
        let sync = CalendarSync::new(deps.store.sync.clone(), deps.directory, deps.clock.clone());
        Self {
            source: deps.source,
            connectors: deps.connectors,
            store: deps.store,
            settings: deps.settings,
            events: deps.events,
            clock: deps.clock,
            nearby: RwLock::new(Vec::new()),
            running: tokio::sync::Mutex::new(()),
            sync,
            state: Mutex::new(SyncState::default()),
        }
    }

    /// The switch, macOS access, each connector and the last syncs.
    pub async fn status(&self) -> CalendarStatus {
        let state = self.state().clone();
        let apple = CalendarProvider::Apple;
        CalendarStatus {
            sync_enabled: self.settings.get().await.calendar.sync_enabled,
            access: self.source.access().await,
            last_synced_at: state.synced_at.get(&apple).copied(),
            last_error: state.errors.get(&apple).cloned(),
            connectors: self.connector_statuses(&state).await,
        }
    }

    /// Turns macOS Calendar sync on: asks macOS for Calendars access the
    /// first time, and syncs at once when it is granted. Denied access
    /// leaves sync off; the status says so, and Settings links to System
    /// Settings.
    pub async fn enable(&self) -> Result<CalendarStatus, CalendarError> {
        let access = self.source.request_access().await?;
        if access == CalendarAccess::Granted {
            self.set_enabled(true).await?;
            self.sync_now().await;
        }
        Ok(self.announce().await)
    }

    /// Turns macOS Calendar sync off. Stored events stay, so meetings keep
    /// their links; its events leave "Coming up" until sync is on again.
    pub async fn disable(&self) -> Result<CalendarStatus, CalendarError> {
        self.set_enabled(false).await?;
        self.refresh_nearby().await;
        Ok(self.announce().await)
    }

    /// Opens System Settings at Calendars access, where a denied permission
    /// is changed. False when there is no such page.
    pub fn open_access_settings(&self) -> bool {
        self.source.open_access_settings()
    }

    /// Syncs every active source now, and returns the status.
    pub async fn refresh(&self) -> CalendarStatus {
        self.sync_if_enabled().await;
        self.status().await
    }

    /// Syncs every active source: macOS Calendar while its switch is on,
    /// each connector while connected. The scheduler's tick; false when no
    /// source is active.
    pub async fn sync_if_enabled(&self) -> bool {
        if self.active_providers().await.is_empty() {
            return false;
        }
        self.sync_now().await;
        self.announce().await;
        true
    }

    /// One sync of every active source, each recorded in the status.
    pub(super) async fn sync_now(&self) {
        let running = self.running.lock().await;
        let settings = self.settings.get().await;
        let active = self.active_providers().await;
        let sources = std::iter::once(&self.source).chain(&self.connectors);
        for source in sources.filter(|source| active.contains(&source.provider())) {
            self.sync_source(source.as_ref(), &settings.profile.emails)
                .await;
        }
        drop(running);
        self.refresh_nearby().await;
    }

    /// Syncs one source and records its outcome.
    async fn sync_source(&self, source: &dyn CalendarSource, profile: &[Email]) {
        let provider = source.provider();
        let result = match source.access().await {
            CalendarAccess::Granted => self.sync.run(source, profile).await,
            _ => Err(SourceError::AccessDenied.into()),
        };
        let mut state = self.state();
        match result {
            Ok(_) => {
                state.synced_at.insert(provider, self.clock.now());
                state.errors.remove(&provider);
            }
            Err(error) => {
                log::warn!(target: "calendar", "{provider} calendar sync failed: {error}");
                state.errors.insert(provider, error.to_string());
            }
        }
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), CalendarError> {
        self.settings
            .update(|settings| settings.calendar.sync_enabled = enabled)
            .await?;
        Ok(())
    }

    /// Emits the current status and returns it.
    pub(super) async fn announce(&self) -> CalendarStatus {
        let status = self.status().await;
        self.events.changed(&status);
        status
    }

    pub(super) fn state(&self) -> MutexGuard<'_, SyncState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
