//! Contract tests: the JSON of every calendar command and of
//! `CalendarChanged`, through the real plugin and ACL, over a scripted
//! macOS Calendar with one iCloud "Work" calendar.

use std::sync::{Arc, mpsc};
use std::time::Duration;

use chrono::TimeDelta;
use deino_core::calendar::{
    AttendeeDirectory, Availability, CalendarAccess, CalendarColor, CalendarDeps, CalendarService,
    CalendarStore, EventStatus, FakeCalendarRepo, FakeCalendarSource, Participant, ParticipantKind,
    SourceAccount, SourceCalendar, SourceEvent, SourceSnapshot,
};
use deino_core::companies::FakeCompanyRepo;
use deino_core::people::FakePeopleRepo;
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_domain::{CalendarProvider, Rsvp};
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Listener, Manager, WebviewWindow};

use super::TauriCalendarEvents;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);
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
        ical_uid: Some("evt-1@deino.dev".into()),
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

fn source() -> Arc<FakeCalendarSource> {
    let source = Arc::new(FakeCalendarSource::new(CalendarProvider::Apple));
    source.set_snapshot(work_snapshot());
    source
}

/// One iCloud account with a "Work" calendar holding the pricing review.
fn work_snapshot() -> SourceSnapshot {
    SourceSnapshot {
        accounts: vec![SourceAccount {
            name: "iCloud".into(),
            calendars: vec![SourceCalendar {
                provider_id: "work".into(),
                title: "Work".into(),
                color: CalendarColor::parse("#34AADC").ok(),
                events: vec![pricing_review()],
            }],
        }],
    }
}

/// An app with the `calendar` plugin over a scripted macOS Calendar.
fn calendar_app() -> App<MockRuntime> {
    calendar_app_over(source())
}

/// An app with the `calendar` plugin over `source`.
fn calendar_app_over(source: Arc<FakeCalendarSource>) -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let settings = tauri::async_runtime::block_on(SettingsService::load(
        Arc::new(FakeSettingsStore::default()),
        Arc::new(RecordedSettingsEvents::default()),
    ))
    .unwrap();
    let repo = Arc::new(FakeCalendarRepo::default());
    let google = Arc::new(FakeCalendarSource::new(CalendarProvider::Google));
    google.set_snapshot(work_snapshot());
    let service = CalendarService::new(CalendarDeps {
        source,
        connectors: vec![google],
        store: CalendarStore::over(&repo),
        directory: AttendeeDirectory::over(
            Arc::new(FakePeopleRepo::default()),
            Arc::new(FakeCompanyRepo::default()),
            clock.clone(),
        ),
        settings: Arc::new(settings),
        events: Arc::new(TauriCalendarEvents::new(app.handle().clone())),
        clock,
    });
    app.manage(Arc::new(service));
    app
}

/// A window whose calendar sync is on.
fn enabled(app: &App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    let window = main_window(app);
    invoke(&window, "plugin:calendar|enable", Value::Null).unwrap();
    window
}

#[test]
fn status_reports_the_switch_access_and_last_sync_in_camel_case() {
    let app = calendar_app();

    let status = invoke(&main_window(&app), "plugin:calendar|status", Value::Null);

    assert_eq!(
        status,
        Ok(json!({
            "syncEnabled": false,
            "access": "not_determined",
            "lastSyncedAt": null,
            "lastError": null,
            "connectors": [{
                "provider": "google",
                "configured": false,
                "connected": false,
                "lastSyncedAt": null,
                "lastError": null,
            }],
        }))
    );
}

#[test]
fn enable_syncs_and_emits_calendar_changed_with_the_status() {
    let app = calendar_app();
    let (sender, received) = mpsc::channel();
    app.listen_any("plugin:calendar:calendar-changed", move |event| {
        sender
            .send(event.payload().to_owned())
            .expect("the test is still listening");
    });

    let status = invoke(&main_window(&app), "plugin:calendar|enable", Value::Null).unwrap();

    let expected = json!({
        "syncEnabled": true,
        "access": "granted",
        "lastSyncedAt": NOW,
        "lastError": null,
        "connectors": [{
            "provider": "google",
            "configured": false,
            "connected": false,
            "lastSyncedAt": null,
            "lastError": null,
        }],
    });
    assert_eq!(status, expected);
    let payload: Value =
        serde_json::from_str(&received.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
    assert_eq!(payload, json!({ "status": expected }));
}

#[test]
fn list_calendars_returns_each_calendar_with_its_account_and_colour() {
    let app = calendar_app();
    let window = enabled(&app);

    let calendars = invoke(&window, "plugin:calendar|list_calendars", Value::Null).unwrap();

    let work = &calendars[0];
    assert_eq!(work["title"], "Work");
    assert_eq!(work["color"], "#34AADC");
    assert_eq!(work["accountName"], "iCloud");
    assert_eq!(work["provider"], "apple");
    assert_eq!(work["visible"], true);
    assert!(work["id"].is_string() && work["accountId"].is_string());
}

#[test]
fn upcoming_takes_a_range_and_returns_events_with_their_link_and_people() {
    let app = calendar_app();
    let window = enabled(&app);
    let range = json!({ "range": { "start": NOW, "end": "2026-10-10T09:00:00Z" } });

    let events = invoke(&window, "plugin:calendar|upcoming", range).unwrap();

    let event = &events[0];
    assert_eq!(event["providerEventId"], "evt-1");
    assert_eq!(event["startAt"], STARTS);
    assert_eq!(event["isAllDay"], false);
    assert_eq!(event["conferenceUrl"], "https://zoom.us/j/123");
    assert_eq!(event["platform"], "zoom");
    assert_eq!(event["myRsvp"], "accepted");
    assert_eq!(event["isExternal"], true);
    assert_eq!(
        event["attendees"][1],
        json!({
            "email": "marcus@brightkite.com",
            "name": null,
            "rsvp": "accepted",
            "isOrganizer": false,
        })
    );
    let id = json!({ "id": event["id"] });
    let fetched = invoke(&window, "plugin:calendar|event", id).unwrap();
    assert_eq!(&fetched, event);
}

#[test]
fn hiding_a_calendar_takes_its_id_and_empties_upcoming() {
    let app = calendar_app();
    let window = enabled(&app);
    let calendars = invoke(&window, "plugin:calendar|list_calendars", Value::Null).unwrap();
    let args = json!({ "id": calendars[0]["id"], "visible": false });

    let hidden = invoke(&window, "plugin:calendar|set_calendar_visible", args).unwrap();

    assert_eq!(hidden["visible"], false);
    let range = json!({ "range": { "start": NOW, "end": "2026-10-10T09:00:00Z" } });
    assert_eq!(
        invoke(&window, "plugin:calendar|upcoming", range),
        Ok(json!([]))
    );
}

#[test]
fn errors_arrive_as_app_errors_with_a_code() {
    let app = calendar_app();
    let window = main_window(&app);
    let missing = json!({ "id": "01926b2c-0000-7000-8000-000000000001" });
    let backwards = json!({ "range": { "start": NOW, "end": "2026-10-08T09:00:00Z" } });

    let event = invoke(&window, "plugin:calendar|event", missing);
    let range = invoke(&window, "plugin:calendar|upcoming", backwards);

    assert_eq!(
        event,
        Err(json!({
            "code": "not_found",
            "message": "calendar event 01926b2c-0000-7000-8000-000000000001 was not found",
        }))
    );
    assert_eq!(range.unwrap_err()["code"], "invalid");
}

#[test]
fn denied_access_keeps_sync_off_and_says_so() {
    let source = source();
    source.answer_prompt_with(CalendarAccess::Denied);
    let app = calendar_app_over(source);
    let window = main_window(&app);

    let status = invoke(&window, "plugin:calendar|enable", Value::Null).unwrap();
    let opened = invoke(&window, "plugin:calendar|open_access_settings", Value::Null);

    assert_eq!(
        (&status["syncEnabled"], &status["access"]),
        (&json!(false), &json!("denied"))
    );
    assert_eq!(
        opened,
        Ok(json!(false)),
        "the scripted source has no settings page"
    );
}

#[test]
fn connect_and_disconnect_take_a_provider_and_return_the_status() {
    let app = calendar_app();
    let window = main_window(&app);
    let google = json!({ "provider": "google" });

    let connected = invoke(&window, "plugin:calendar|connect", google.clone()).unwrap();
    let disconnected = invoke(&window, "plugin:calendar|disconnect", google).unwrap();
    let missing = invoke(
        &window,
        "plugin:calendar|connect",
        json!({ "provider": "microsoft" }),
    );

    assert_eq!(connected["connectors"][0]["connected"], true);
    assert_eq!(connected["connectors"][0]["lastSyncedAt"], NOW);
    assert_eq!(disconnected["connectors"][0]["connected"], false);
    assert_eq!(
        missing,
        Err(json!({ "code": "invalid", "message": "there is no microsoft calendar connector" }))
    );
}
