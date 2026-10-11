//! Contract tests of ORG-03's commands: the JSON of `meeting_event`,
//! `event_suggestions`, `link_event`, `unlink_event` and
//! `related_meetings`, through the real plugin and ACL, over a SQLite
//! database with one meeting and a design review on the calendar.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use deino_core::calendar::linking::{
    EventAgenda, EventAttendance, EventLinkDeps, EventLinkService, RelatedService,
    SqliteRelatedRepo,
};
use deino_core::calendar::{
    AttendeeDirectory, CalendarAccount, CalendarAccountRepo, CalendarError, CalendarEvent,
    CalendarEventRepo, EventAttendee, SqliteCalendarRepo, TimeRange,
};
use deino_core::companies::SqliteCompanyRepo;
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::people::SqlitePeopleRepo;
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_db::Database;
use deino_domain::{CalendarAccountId, CalendarEventId, CalendarProvider, Rsvp, SeriesId};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewWindow};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const STARTS: &str = "2026-10-09T10:00:00Z";

/// The calendar: the one event, whatever the range.
struct OneEvent(CalendarEvent);

#[async_trait]
impl EventAgenda for OneEvent {
    async fn events(&self, _range: TimeRange) -> Result<Vec<CalendarEvent>, CalendarError> {
        Ok(vec![self.0.clone()])
    }
}

/// The 10:00 design review, organised by Ada, in the weekly series.
fn design_review(account: CalendarAccountId, clock: &FakeClock) -> CalendarEvent {
    let starts: DateTime<Utc> = at(STARTS);
    CalendarEvent {
        id: CalendarEventId::generate(clock),
        account_id: account,
        calendar_id: None,
        provider_event_id: "evt-1".into(),
        ical_uid: None,
        series_id: Some(SeriesId::new("design-weekly")),
        title: "Design review".into(),
        description: "Walk through the new onboarding".into(),
        start_at: starts,
        end_at: starts + TimeDelta::minutes(30),
        is_all_day: false,
        conference_url: None,
        platform: None,
        organizer_email: Some("ada@acme.com".into()),
        attendees: vec![EventAttendee {
            email: Some("ada@acme.com".into()),
            name: Some("Ada Lovelace".into()),
            rsvp: Some(Rsvp::Accepted),
            is_organizer: true,
        }],
        my_rsvp: Some(Rsvp::Accepted),
        is_external: true,
        record_override: None,
        created_at: starts,
        updated_at: starts,
    }
}

/// Stores the account, the event and two meetings: this week's and last
/// week's, already linked to the series.
async fn seed_calendar(database: &Database, clock: &FakeClock) -> CalendarEvent {
    let calendar = SqliteCalendarRepo::new(database.clone());
    let account = CalendarAccount {
        id: CalendarAccountId::generate(clock),
        provider: CalendarProvider::Apple,
        name: "On My Mac".into(),
        created_at: at(STARTS),
    };
    calendar.add_account(&account).await.unwrap();
    let event = design_review(account.id, clock);
    calendar.upsert_event(&event).await.unwrap();
    seed::meeting(
        database,
        "meeting-now",
        "New meeting",
        at("2026-10-09T10:01:00Z"),
    )
    .await;
    seed::meeting(
        database,
        "meeting-last",
        "Design review",
        at("2026-10-02T10:00:00Z"),
    )
    .await;
    sqlx::query("UPDATE meetings SET series_id = 'design-weekly' WHERE id = 'meeting-last'")
        .execute(database.writer())
        .await
        .unwrap();
    event
}

async fn link_service(
    database: &Database,
    event: CalendarEvent,
    clock: Arc<FakeClock>,
) -> EventLinkService {
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let settings = SettingsService::load(
        Arc::new(FakeSettingsStore::default()),
        Arc::new(RecordedSettingsEvents::default()),
    )
    .await
    .unwrap();
    let directory = AttendeeDirectory::new(
        people.clone(),
        Arc::new(SqliteCompanyRepo::new(database.clone())),
        clock.clone(),
    );
    EventLinkService::new(EventLinkDeps {
        meetings: meetings.clone(),
        lifecycle: meetings,
        events: Arc::new(SqliteCalendarRepo::new(database.clone())),
        agenda: Arc::new(OneEvent(event)),
        attendance: EventAttendance::new(directory, people.clone(), people, Arc::new(settings)),
        clock,
    })
}

/// An app with the `calendar` plugin over the seeded database; the event.
fn linking_app() -> (App<MockRuntime>, CalendarEvent) {
    let app = mock_app(mock_builder().plugin(super::init()));
    let clock = Arc::new(FakeClock::new(at("2026-10-09T10:02:00Z")));
    let (links, related, event) = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        let event = seed_calendar(&database, &clock).await;
        let links = link_service(&database, event.clone(), clock.clone()).await;
        let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
        let related = RelatedService::new(Arc::new(SqliteRelatedRepo::new(database)), people);
        (links, related, event)
    });
    app.manage(Arc::new(links));
    app.manage(Arc::new(related));
    (app, event)
}

fn meeting(id: &str) -> Value {
    json!({ "meetingId": id })
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:calendar|{command}"), args)
}

#[test]
fn link_event_returns_the_event_title_and_attendee_counts_in_camel_case() {
    let (app, event) = linking_app();
    let window = main_window(&app);
    let args = json!({ "meetingId": "meeting-now", "eventId": event.id });

    let change = call(&window, "link_event", args).unwrap();

    assert_eq!(change["meetingId"], "meeting-now");
    assert_eq!(change["title"], "Design review");
    assert_eq!(change["attendeesAdded"], 1);
    assert_eq!(change["attendeesRemoved"], 0);
    assert_eq!(change["event"]["id"], json!(event.id));
    assert_eq!(
        change["event"]["description"],
        "Walk through the new onboarding"
    );
    let linked = call(&window, "meeting_event", meeting("meeting-now")).unwrap();
    assert_eq!(linked["title"], "Design review");
}

#[test]
fn an_unlinked_meeting_has_a_null_event_and_suggestions() {
    let (app, event) = linking_app();
    let window = main_window(&app);

    let linked = call(&window, "meeting_event", meeting("meeting-now"));
    let suggested = call(&window, "event_suggestions", meeting("meeting-now")).unwrap();

    assert_eq!(linked, Ok(Value::Null));
    assert_eq!(suggested[0]["id"], json!(event.id));
    assert_eq!(suggested[0]["startAt"], STARTS);
}

#[test]
fn unlink_event_returns_a_null_event() {
    let (app, event) = linking_app();
    let window = main_window(&app);
    call(
        &window,
        "link_event",
        json!({ "meetingId": "meeting-now", "eventId": event.id }),
    )
    .unwrap();

    let change = call(&window, "unlink_event", meeting("meeting-now")).unwrap();

    assert_eq!(
        change,
        json!({
            "meetingId": "meeting-now",
            "event": null,
            "title": null,
            "attendeesAdded": 0,
            "attendeesRemoved": 1,
        })
    );
}

#[test]
fn related_meetings_lists_the_previous_meeting_of_the_series() {
    let (app, event) = linking_app();
    let window = main_window(&app);
    call(
        &window,
        "link_event",
        json!({ "meetingId": "meeting-now", "eventId": event.id }),
    )
    .unwrap();

    let related = call(&window, "related_meetings", meeting("meeting-now")).unwrap();

    assert_eq!(
        related,
        json!({
            "previous": {
                "id": "meeting-last",
                "title": "Design review",
                "happenedAt": "2026-10-02T10:00:00Z",
                "by": "series",
            },
            "others": [],
        })
    );
}

#[test]
fn an_unknown_meeting_or_event_is_not_found() {
    let (app, event) = linking_app();
    let window = main_window(&app);

    let no_meeting = call(&window, "event_suggestions", meeting("meeting-missing")).unwrap_err();
    let args = json!({ "meetingId": "meeting-missing", "eventId": event.id });
    let no_meeting_link = call(&window, "link_event", args).unwrap_err();

    assert_eq!(no_meeting["code"], "not_found");
    assert_eq!(no_meeting_link["code"], "not_found");
    assert_eq!(
        no_meeting["message"],
        "meeting meeting-missing was not found"
    );
}
