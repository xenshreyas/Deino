//! Contract tests of ORG-02's commands: the JSON of `record_plan`,
//! `choose_record`, `join`, `reminder`, `dismiss_reminder`,
//! `set_open_note` and `last_meeting`, through the real plugin and ACL,
//! over a SQLite calendar with one pricing review on Zoom.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_core::calendar::join::{CallOpener, EventRecorder, JoinService, PortError};
use deino_core::calendar::linking::{LastMeetingService, RelatedService, SqliteRelatedRepo};
use deino_core::calendar::rules::{AutoRecordDeps, AutoRecordService};
use deino_core::calendar::ticker::{OpenNote, ReminderWindow};
use deino_core::calendar::{
    AttendeeDirectory, Availability, CalendarDeps, CalendarEvent, CalendarService, CalendarStore,
    EventStatus, FakeCalendarSource, Participant, ParticipantKind, RecordedCalendarEvents,
    SourceAccount, SourceCalendar, SourceEvent, SourceSnapshot, SqliteCalendarRepo, TimeRange,
};
use deino_core::companies::SqliteCompanyRepo;
use deino_core::people::SqlitePeopleRepo;
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_domain::{CalendarProvider, Email, MeetingId, Rsvp};
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewWindow};

use super::ReminderCard;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-09T09:00:00Z";
const STARTS: &str = "2026-10-09T10:00:00Z";

/// The 10:00 pricing review on Zoom, with the user and a guest.
fn pricing_review() -> SourceEvent {
    let person = |email: &str, is_self: bool| Participant {
        email: Some(email.into()),
        name: None,
        rsvp: Some(Rsvp::Accepted),
        kind: ParticipantKind::Person,
        is_self,
    };
    SourceEvent {
        provider_event_id: "evt-1".into(),
        ical_uid: None,
        series_key: None,
        title: "Pricing review".into(),
        notes: None,
        location: Some("https://zoom.us/j/123".into()),
        url: None,
        conference_url: None,
        start: at(STARTS),
        end: at(STARTS) + TimeDelta::minutes(30),
        all_day: false,
        status: EventStatus::Confirmed,
        kind: None,
        availability: Availability::Busy,
        organizer: None,
        attendees: vec![
            person("maya@deino.dev", true),
            person("marcus@brightkite.com", false),
        ],
    }
}

/// Starts "recording" at once and keeps the titles it recorded.
#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

#[async_trait]
impl EventRecorder for Recorder {
    async fn record(&self, event: &CalendarEvent) -> Result<MeetingId, PortError> {
        self.0.lock().unwrap().push(event.title.clone());
        Ok(MeetingId::new("meeting-1"))
    }
}

/// Opens nothing, successfully.
struct Opener;

#[async_trait]
impl CallOpener for Opener {
    async fn open_call(&self, _url: &str) -> Result<(), PortError> {
        Ok(())
    }
}

/// A window that only counts.
#[derive(Default)]
struct Window(Mutex<u32>);

impl ReminderWindow for Window {
    fn show(&self, _event: &CalendarEvent) {
        *self.0.lock().unwrap() += 1;
    }

    fn close(&self) {}
}

/// The services the commands use, over one database.
struct Services {
    calendar: Arc<CalendarService>,
    rules: AutoRecordService,
    history: LastMeetingService,
}

/// Settings whose profile has the user's address.
async fn settings() -> Arc<SettingsService> {
    let settings = Arc::new(
        SettingsService::load(
            Arc::new(FakeSettingsStore::default()),
            Arc::new(RecordedSettingsEvents::default()),
        )
        .await
        .unwrap(),
    );
    settings
        .update(|s| s.profile.emails = vec![Email::parse("maya@deino.dev").unwrap()])
        .await
        .unwrap();
    settings
}

/// A synced calendar with the pricing review, and the services over it.
async fn services(recorder: Arc<Recorder>) -> (Services, JoinService) {
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let database = test_db().await;
    let settings = settings().await;
    let repo = Arc::new(SqliteCalendarRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let calendar = Arc::new(CalendarService::new(CalendarDeps {
        source: source(),
        connectors: Vec::new(),
        store: CalendarStore::over(&repo),
        directory: AttendeeDirectory::over(
            people.clone(),
            Arc::new(SqliteCompanyRepo::new(database.clone())),
            clock.clone(),
        ),
        settings: settings.clone(),
        events: Arc::new(RecordedCalendarEvents::default()),
        clock: clock.clone(),
    }));
    calendar.enable().await.unwrap();
    let rules = AutoRecordService::new(AutoRecordDeps {
        calendar: calendar.clone(),
        events: repo.clone(),
        series: repo,
        settings,
        clock,
    });
    let related = RelatedService::new(Arc::new(SqliteRelatedRepo::new(database)), people.clone());
    let history =
        LastMeetingService::new(Arc::new(related), people.clone(), people.clone(), people);
    let joins = JoinService::new(calendar.clone(), Arc::new(Opener), recorder);
    let services = Services {
        calendar,
        rules,
        history,
    };
    (services, joins)
}

/// macOS Calendar with one iCloud "Work" calendar holding the pricing review.
fn source() -> Arc<FakeCalendarSource> {
    let source = Arc::new(FakeCalendarSource::new(CalendarProvider::Apple));
    source.set_snapshot(SourceSnapshot {
        accounts: vec![SourceAccount {
            name: "iCloud".into(),
            calendars: vec![SourceCalendar {
                provider_id: "work".into(),
                title: "Work".into(),
                color: None,
                events: vec![pricing_review()],
            }],
        }],
    });
    source
}

/// The app, the recorder, and the pricing review.
struct Harness {
    app: App<MockRuntime>,
    recorder: Arc<Recorder>,
    event: CalendarEvent,
}

impl Harness {
    fn id(&self) -> String {
        self.event.id.to_string()
    }
}

fn harness() -> Harness {
    let recorder = Arc::new(Recorder::default());
    let (services, joins) = tauri::async_runtime::block_on(services(recorder.clone()));
    let range = TimeRange {
        start: at(NOW),
        end: at(NOW) + TimeDelta::days(1),
    };
    let event = tauri::async_runtime::block_on(services.calendar.upcoming(range))
        .unwrap()
        .remove(0);
    let app = mock_app(mock_builder().plugin(super::init()));
    app.manage(services.calendar);
    app.manage(Arc::new(services.rules));
    app.manage(Arc::new(services.history));
    app.manage(Arc::new(joins));
    app.manage(Arc::new(ReminderCard::new(Arc::new(Window::default()))));
    app.manage(Arc::new(OpenNote::default()));
    Harness {
        app,
        recorder,
        event,
    }
}

fn window(harness: &Harness) -> WebviewWindow<MockRuntime> {
    main_window(&harness.app)
}

#[test]
fn record_plan_lists_each_event_with_its_decision() {
    let h = harness();
    let range = json!({ "range": { "start": NOW, "end": "2026-10-10T09:00:00Z" } });

    let plan = invoke(&window(&h), "plugin:calendar|record_plan", range).unwrap();

    assert_eq!(plan.as_array().map(Vec::len), Some(1));
    assert_eq!(plan[0]["event"]["id"], h.id().as_str());
    assert_eq!(plan[0]["event"]["title"], "Pricing review");
    assert_eq!(
        plan[0]["decision"],
        json!({ "record": false, "reason": "audienceOff" })
    );
}

#[test]
fn choose_record_returns_the_new_decision() {
    let h = harness();
    let args = json!({ "id": h.id(), "choice": { "record": true, "scope": "allOccurrences" } });

    let decision = invoke(&window(&h), "plugin:calendar|choose_record", args).unwrap();

    assert_eq!(decision, json!({ "record": true, "reason": "eventChoice" }));
    let cleared = json!({ "id": h.id(), "choice": { "record": null, "scope": "thisEvent" } });
    let back = invoke(&window(&h), "plugin:calendar|choose_record", cleared).unwrap();
    assert_eq!(back, json!({ "record": false, "reason": "audienceOff" }));
}

#[test]
fn join_returns_the_recording_meeting_or_null() {
    let h = harness();
    let record = json!({ "id": h.id(), "mode": "joinAndRecord" });
    let joined = invoke(&window(&h), "plugin:calendar|join", record).unwrap();
    assert_eq!(joined, json!("meeting-1"));
    assert_eq!(*h.recorder.0.lock().unwrap(), ["Pricing review"]);

    let only = json!({ "id": h.id(), "mode": "joinOnly" });
    assert_eq!(
        invoke(&window(&h), "plugin:calendar|join", only),
        Ok(Value::Null)
    );
}

#[test]
fn join_fails_with_not_found_for_an_unknown_event() {
    let h = harness();
    let args = json!({ "id": "0199c0de-0000-7000-8000-000000000000", "mode": "recordOnly" });

    let error = invoke(&window(&h), "plugin:calendar|join", args).unwrap_err();

    assert_eq!(error["code"], "not_found");
}

#[test]
fn the_reminder_is_the_cards_event_until_dismissed() {
    let h = harness();
    let w = window(&h);
    assert_eq!(
        invoke(&w, "plugin:calendar|reminder", Value::Null),
        Ok(Value::Null)
    );

    let card = h.app.state::<Arc<ReminderCard>>();
    card.show(&h.event);
    let shown = invoke(&w, "plugin:calendar|reminder", Value::Null).unwrap();
    assert_eq!(shown["id"], h.id().as_str());
    assert_eq!(shown["title"], "Pricing review");

    assert_eq!(
        invoke(&w, "plugin:calendar|dismiss_reminder", Value::Null),
        Ok(Value::Null)
    );
    assert_eq!(card.current(), None);
}

#[test]
fn set_open_note_records_the_event_on_screen() {
    let h = harness();
    let open = json!({ "event": h.id() });

    assert_eq!(
        invoke(&window(&h), "plugin:calendar|set_open_note", open),
        Ok(Value::Null)
    );
    let note = h.app.state::<Arc<OpenNote>>();
    assert_eq!(note.get().map(|id| id.to_string()), Some(h.id()));

    let closed = json!({ "event": null });
    invoke(&window(&h), "plugin:calendar|set_open_note", closed).unwrap();
    assert_eq!(note.get(), None);
}

#[test]
fn last_meeting_is_null_without_earlier_meetings() {
    let h = harness();
    let args = json!({ "id": h.id() });

    assert_eq!(
        invoke(&window(&h), "plugin:calendar|last_meeting", args),
        Ok(Value::Null)
    );
}
