//! A [`CalendarService`] over the real schema with `events` synced, for the
//! ORG-02 tests (auto-record rules, Join).

use std::sync::Arc;

use deino_db::test_db;
use deino_domain::{CalendarProvider, Email};

use super::test_events::{ME, clock, snapshot};
use super::{
    AttendeeDirectory, CalendarDeps, CalendarService, CalendarStore, FakeCalendarSource,
    RecordedCalendarEvents, SourceEvent, SqliteCalendarRepo,
};
use crate::companies::FakeCompanyRepo;
use crate::people::FakePeopleRepo;
use crate::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};

/// The service, its repository and the settings it reads.
pub(super) struct SyncedCalendar {
    pub(super) service: Arc<CalendarService>,
    pub(super) repo: Arc<SqliteCalendarRepo>,
    pub(super) settings: Arc<SettingsService>,
}

/// Sync on, `events` in the user's Work calendar, the profile's address [`ME`].
pub(super) async fn synced_calendar(events: Vec<SourceEvent>) -> SyncedCalendar {
    let clock = Arc::new(clock());
    let source = Arc::new(FakeCalendarSource::new(CalendarProvider::Apple));
    source.set_snapshot(snapshot(events));
    let settings = SettingsService::load(
        Arc::new(FakeSettingsStore::default()),
        Arc::new(RecordedSettingsEvents::default()),
    )
    .await
    .unwrap();
    let settings = Arc::new(settings);
    settings
        .update(|settings| settings.profile.emails = vec![Email::parse(ME).unwrap()])
        .await
        .unwrap();
    let repo = Arc::new(SqliteCalendarRepo::new(test_db().await));
    let service = Arc::new(CalendarService::new(CalendarDeps {
        source,
        connectors: Vec::new(),
        store: CalendarStore::over(&repo),
        directory: AttendeeDirectory::over(
            Arc::new(FakePeopleRepo::default()),
            Arc::new(FakeCompanyRepo::default()),
            clock.clone(),
        ),
        settings: settings.clone(),
        events: Arc::new(RecordedCalendarEvents::default()),
        clock,
    }));
    service.enable().await.unwrap();
    SyncedCalendar {
        service,
        repo,
        settings,
    }
}
