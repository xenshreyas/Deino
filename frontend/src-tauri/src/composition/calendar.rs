//! Wiring for calendar sync (ORG-01): macOS Calendar through `EventKit`, the
//! Google Calendar and Microsoft Graph connectors over `deino-net` scoped
//! clients, the SQLite calendar, people and company repositories, the sync
//! scheduler, and the `CalendarChanged` event adapter. Then event linking and
//! related meetings (ORG-03) over the same calendar, and ORG-02's rules,
//! Join, "Review last meeting" and the meeting ticker (reminders and the
//! start-time trigger).

use std::sync::Arc;

use deino_connect::calendar::{
    CalendarApi, ConnectorParts, ConnectorSource, Credentials, GoogleCalendar, MicrosoftCalendar,
    SettingsCredentials,
};
use deino_core::calendar::join::JoinService;
use deino_core::calendar::linking::{
    EventAttendance, EventLinkDeps, EventLinkService, LastMeetingService, RelatedService,
    SqliteRelatedRepo,
};
use deino_core::calendar::rules::{AutoRecordDeps, AutoRecordService};
use deino_core::calendar::ticker::{MeetingTicker, OpenNote, TickerDeps};
use deino_core::calendar::{
    AttendeeDirectory, CalendarDeps, CalendarService, CalendarSource, CalendarStore,
    SqliteCalendarRepo, SyncScheduler,
};
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

use super::attach::Phase;
use super::browser::SystemBrowser;
use super::network::SettingsServicePolicy;
use crate::features::calendar::{
    LiveRecorder, RecorderPhase, ReminderCard, SystemCallOpener, TauriCalendarEvents,
    TauriReminderWindow,
};

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

super::attach::attach!(
    "calendar",
    50,
    Phase::Background(|cx| {
        attach_calendar(cx.app, cx.database, cx.clock);
    })
);

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
    attach_ticker(app, &service, clock, cancel.child_token());
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
    let related = Arc::new(RelatedService::new(Arc::new(related), people.clone()));
    app.manage(Arc::new(LastMeetingService::new(
        related.clone(),
        people.clone(),
        people.clone(),
        people,
    )));
    app.manage(related);
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

/// Starts the meeting ticker (ORG-02: the reminder window, and recording an
/// open note's event at its start) until `cancel`, the sync scheduler's
/// child, fires at exit. Manages `Arc<OpenNote>` and the window. Needs
/// [`attach_rules`] to have run.
fn attach_ticker(
    app: &AppHandle,
    calendar: &Arc<CalendarService>,
    clock: &Arc<dyn Clock>,
    cancel: CancellationToken,
) {
    let state = |app: &AppHandle| {
        let rules = Arc::clone(&*app.try_state::<Arc<AutoRecordService>>()?);
        let joins = Arc::clone(&*app.try_state::<Arc<JoinService>>()?);
        let settings = Arc::clone(&*app.try_state::<Arc<SettingsService>>()?);
        Some((rules, joins, settings))
    };
    let Some((rules, joins, settings)) = state(app) else {
        return log::error!(target: "calendar", "the rules are not wired; meeting reminders are off");
    };
    let window = Arc::new(ReminderCard::new(Arc::new(TauriReminderWindow::new(
        app.clone(),
    ))));
    let open_note = Arc::new(OpenNote::default());
    let ticker = Arc::new(MeetingTicker::new(TickerDeps {
        calendar: Arc::clone(calendar),
        rules,
        joins,
        settings,
        window: window.clone(),
        recorder: Arc::new(RecorderPhase::new(app.clone())),
        open_note: open_note.clone(),
        clock: Arc::clone(clock),
    }));
    // Owned for the app's lifetime; `CalendarSchedule::stop` cancels it.
    tauri::async_runtime::spawn(ticker.run(cancel));
    app.manage(window);
    app.manage(open_note);
}
