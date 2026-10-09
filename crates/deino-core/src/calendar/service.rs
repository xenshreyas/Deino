//! [`CalendarService`]: calendar sync as the app sees it.
//!
//! This file holds the sync lifecycle: the switch (`enable` asks macOS for
//! access, §3C), `refresh` and the scheduler's `sync_if_enabled`, and the
//! status Settings shows. `reads.rs` holds what the UI lists. Syncs never
//! overlap: one lock serialises the scheduler, wake-ups and the Refresh
//! button.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};

use chrono::{DateTime, Utc};
use deino_domain::Clock;

use super::batch::SyncReport;
use super::entities::AttendeeDirectory;
use super::sync::CalendarSync;
use super::{
    CalendarAccess, CalendarAccountRepo, CalendarError, CalendarEvent, CalendarEventRepo,
    CalendarEvents, CalendarRepo, CalendarSource, CalendarStatus, CalendarSyncRepo,
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
    /// The stored calendars.
    pub store: CalendarStore,
    /// Where synced attendees become people and companies.
    pub directory: AttendeeDirectory,
    /// Holds the sync switch and the hidden calendars.
    pub settings: Arc<SettingsService>,
    /// Where `CalendarChanged` goes.
    pub events: Arc<dyn CalendarEvents>,
    /// The time.
    pub clock: Arc<dyn Clock>,
}

/// The outcome of the last sync this session.
#[derive(Clone, Debug, Default)]
struct SyncState {
    last_synced_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

/// Syncs the user's calendars and answers what the UI asks about them.
pub struct CalendarService {
    pub(super) source: Arc<dyn CalendarSource>,
    pub(super) store: CalendarStore,
    pub(super) settings: Arc<SettingsService>,
    pub(super) events: Arc<dyn CalendarEvents>,
    pub(super) clock: Arc<dyn Clock>,
    /// The visible events around now, for synchronous lookups (`events_near`).
    pub(super) nearby: RwLock<Vec<CalendarEvent>>,
    sync: CalendarSync,
    state: Mutex<SyncState>,
    running: tokio::sync::Mutex<()>,
}

impl CalendarService {
    /// The service over its ports.
    pub fn new(deps: CalendarDeps) -> Self {
        let sync = CalendarSync::new(deps.store.sync.clone(), deps.directory, deps.clock.clone());
        Self {
            source: deps.source,
            store: deps.store,
            settings: deps.settings,
            events: deps.events,
            clock: deps.clock,
            nearby: RwLock::new(Vec::new()),
            sync,
            state: Mutex::new(SyncState::default()),
            running: tokio::sync::Mutex::new(()),
        }
    }

    /// The switch, macOS access and the last sync's outcome.
    pub async fn status(&self) -> CalendarStatus {
        let state = self.state().clone();
        CalendarStatus {
            sync_enabled: self.settings.get().await.calendar.sync_enabled,
            access: self.source.access().await,
            last_synced_at: state.last_synced_at,
            last_error: state.last_error,
        }
    }

    /// Turns sync on: asks macOS for Calendars access the first time, and
    /// syncs at once when it is granted. Denied access leaves sync off; the
    /// status says so, and Settings links to System Settings.
    pub async fn enable(&self) -> Result<CalendarStatus, CalendarError> {
        let access = self.source.request_access().await?;
        if access == CalendarAccess::Granted {
            self.set_enabled(true).await?;
            self.sync_now().await;
        }
        Ok(self.announce().await)
    }

    /// Turns sync off. Stored events stay, so meetings keep their links;
    /// "Coming up" empties until sync is on again.
    pub async fn disable(&self) -> Result<CalendarStatus, CalendarError> {
        self.set_enabled(false).await?;
        self.write_nearby(Vec::new());
        Ok(self.announce().await)
    }

    /// Opens System Settings at Calendars access, where a denied permission
    /// is changed. False when there is no such page.
    pub fn open_access_settings(&self) -> bool {
        self.source.open_access_settings()
    }

    /// Syncs now if sync is on, and returns the status.
    pub async fn refresh(&self) -> CalendarStatus {
        self.sync_if_enabled().await;
        self.status().await
    }

    /// Syncs if sync is on and macOS still grants access; the scheduler's
    /// tick. `None` when nothing ran.
    pub async fn sync_if_enabled(&self) -> Option<SyncReport> {
        if !self.settings.get().await.calendar.sync_enabled {
            return None;
        }
        let report = self.sync_now().await;
        self.announce().await;
        report
    }

    /// One sync, recorded in the status; `None` when it failed.
    async fn sync_now(&self) -> Option<SyncReport> {
        let _running = self.running.lock().await;
        let result = match self.source.access().await {
            CalendarAccess::Granted => {
                let profile = self.settings.get().await.profile;
                self.sync.run(self.source.as_ref(), &profile.emails).await
            }
            _ => Err(super::SourceError::AccessDenied.into()),
        };
        let now = self.clock.now();
        let report = match result {
            Ok(report) => {
                *self.state() = SyncState {
                    last_synced_at: Some(now),
                    last_error: None,
                };
                Some(report)
            }
            Err(error) => {
                log::warn!(target: "calendar", "calendar sync failed: {error}");
                self.state().last_error = Some(error.to_string());
                None
            }
        };
        self.refresh_nearby().await;
        report
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

    fn state(&self) -> MutexGuard<'_, SyncState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
