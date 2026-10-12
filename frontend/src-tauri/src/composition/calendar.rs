//! Wiring for calendar sync (ORG-01): macOS Calendar through `EventKit`, the
//! Google Calendar and Microsoft Graph connectors over `deino-net` scoped
//! clients, the SQLite calendar, people and company repositories, the sync
//! scheduler, and the `CalendarChanged` event adapter. Then event linking and
//! related meetings (ORG-03) over the same calendar.

use std::sync::Arc;

use deino_connect::calendar::{
    CalendarApi, ConnectorParts, ConnectorSource, Credentials, GoogleCalendar, MicrosoftCalendar,
    SettingsCredentials,
};
use deino_core::calendar::join::JoinService;
use deino_core::calendar::linking::{
    EventAttendance, EventLinkDeps, EventLinkService, RelatedService, SqliteRelatedRepo,
};
use deino_core::calendar::{
    AttendeeDirectory, CalendarDeps, CalendarService, CalendarSource, CalendarStore,
    SqliteCalendarRepo, SyncScheduler,
};
use deino_core::calendar::rules::{AutoRecordDeps, AutoRecordService};
use deino_core::companies::SqliteCompanyRepo;
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::people::SqlitePeopleRepo;
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::Clock;
use deino_eventkit::EventKitSource;
use deino_net::Net;
use deino_secrets::SecretStore;
use tauri::{AppHandle, Manager};
use tokio_util::sync::CancellationToken;

use super::browser::SystemBrowser;
use super::network::SettingsServicePolicy;
use crate::features::calendar::{LiveRecorder, SystemCallOpener, TauriCalendarEvents};

/// Stops the calendar sync scheduler at exit.
pub(crate) struct CalendarSchedule(CancellationToken);

impl CalendarSchedule {
    /// Ends the scheduler's loop; a sync in progress finishes first.
    pub(crate) fn stop(&self) {
        self.0.cancel();
    }
}

/// What the connectors share: the network, the Keychain, the network
/// switches, the user's app registrations and the clock.
struct ConnectorKit {
    net: Net,
    secrets: Arc<dyn SecretStore>,
    switch: Arc<SettingsServicePolicy>,
    credentials: Arc<dyn Credentials>,
    clock: Arc<dyn Clock>,
}

impl ConnectorKit {
    /// The kit from Tauri state; `None` when the network or the Keychain
    /// was not wired (the connectors are then simply absent).
    fn from_state(
        app: &AppHandle,
        settings: &Arc<SettingsService>,
        clock: &Arc<dyn Clock>,
    ) -> Option<Self> {
        let net = app.try_state::<Net>()?.inner().clone();
        let secrets = Arc::clone(&*app.try_state::<Arc<dyn SecretStore>>()?);
        let switch = Arc::clone(&*app.try_state::<Arc<SettingsServicePolicy>>()?);
        let credentials = Arc::new(SettingsCredentials::new(
            Arc::clone(settings),
            secrets.clone(),
        ));
        Some(Self {
            net,
            secrets,
            switch,
            credentials,
            clock: Arc::clone(clock),
        })
    }

    /// A connector for `api`, reaching only its service's hosts.
    fn connector(&self, api: Arc<dyn CalendarApi>) -> Arc<dyn CalendarSource> {
        Arc::new(ConnectorSource::new(ConnectorParts {
            http: Arc::new(self.net.scoped(api.service())),
            api,
            credentials: Arc::clone(&self.credentials),
            secrets: Arc::clone(&self.secrets),
            switch: self.switch.clone(),
            browser: Arc::new(SystemBrowser),
            clock: Arc::clone(&self.clock),
        }))
    }
}

/// Builds the calendar service over `database`, starts its scheduler (which
/// syncs only active sources) and manages `Arc<CalendarService>` and the
/// scheduler's [`CalendarSchedule`]. Needs the settings and the network.
pub(crate) fn attach_calendar(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let Some(settings) = app
        .try_state::<Arc<SettingsService>>()
        .map(|s| Arc::clone(&s))
    else {
        return log::error!(target: "calendar", "settings are not loaded; calendar sync is off");
    };
    let connectors = ConnectorKit::from_state(app, &settings, clock).map_or_else(Vec::new, |kit| {
        vec![
            kit.connector(Arc::new(GoogleCalendar)),
            kit.connector(Arc::new(MicrosoftCalendar)),
        ]
    });
    let source = Arc::new(EventKitSource::new());
    let repo = Arc::new(SqliteCalendarRepo::new(database.clone()));
    let service = Arc::new(CalendarService::new(CalendarDeps {
        source: source.clone(),
        connectors,
        store: CalendarStore::over(&repo),
        directory: directory(database, clock),
        settings,
        events: Arc::new(TauriCalendarEvents::new(app.clone())),
        clock: Arc::clone(clock),
    }));
    attach_linking(app, database, &service, clock);
    attach_rules(app, &repo, &service, clock);
    let (scheduler, triggers, cancel) = SyncScheduler::new(Arc::clone(&service));
    source.watch(triggers);
    // Owned for the app's lifetime; `CalendarSchedule::stop` ends it at exit.
    tauri::async_runtime::spawn(scheduler.run());
    app.manage(service);
    app.manage(CalendarSchedule(cancel));
}

/// People and companies from calendar invitees.
fn directory(database: &Database, clock: &Arc<dyn Clock>) -> AttendeeDirectory {
    AttendeeDirectory::over(
        Arc::new(SqlitePeopleRepo::new(database.clone())),
        Arc::new(SqliteCompanyRepo::new(database.clone())),
        Arc::clone(clock),
    )
}

/// Manages `Arc<EventLinkService>` (the meeting header's picker, and
/// auto-link when recording starts) and `Arc<RelatedService>`, over the
/// calendar's visible events. Needs the settings (the profile's addresses).
fn attach_linking(
    app: &AppHandle,
    database: &Database,
    calendar: &Arc<CalendarService>,
    clock: &Arc<dyn Clock>,
) {
    let Some(settings) = app
        .try_state::<Arc<SettingsService>>()
        .map(|s| Arc::clone(&s))
    else {
        return log::error!(target: "calendar", "settings are not loaded; event linking is off");
    };
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let attendance = EventAttendance::new(
        directory(database, clock),
        people.clone(),
        people.clone(),
        settings,
    );
    app.manage(Arc::new(EventLinkService::new(EventLinkDeps {
        meetings: meetings.clone(),
        lifecycle: meetings,
        events: Arc::new(SqliteCalendarRepo::new(database.clone())),
        agenda: calendar.clone(),
        attendance,
        clock: Arc::clone(clock),
    })));
    let related = SqliteRelatedRepo::new(database.clone());
    app.manage(Arc::new(RelatedService::new(Arc::new(related), people)));
}

/// Manages `Arc<AutoRecordService>` (ORG-02's rules over the visible
/// events) and `Arc<JoinService>` (Join & record over the live recorder).
fn attach_rules(
    app: &AppHandle,
    repo: &Arc<SqliteCalendarRepo>,
    calendar: &Arc<CalendarService>,
    clock: &Arc<dyn Clock>,
) {
    let Some(settings) = app
        .try_state::<Arc<SettingsService>>()
        .map(|s| Arc::clone(&s))
    else {
        return log::error!(target: "calendar", "settings are not loaded; auto-record is off");
    };
    app.manage(Arc::new(AutoRecordService::new(AutoRecordDeps {
        calendar: Arc::clone(calendar),
        events: repo.clone(),
        series: repo.clone(),
        settings,
        clock: Arc::clone(clock),
    })));
    app.manage(Arc::new(JoinService::new(
        calendar.clone(),
        Arc::new(SystemCallOpener),
        Arc::new(LiveRecorder::new(app.clone())),
    )));
}
