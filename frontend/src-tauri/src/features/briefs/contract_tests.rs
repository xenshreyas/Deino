//! Contract tests: the JSON of the briefs commands, through the real plugin,
//! ACL and error mapping, over the seeded fixture library.

use std::sync::Arc;

use deino_core::briefs::{
    BriefBullet, BriefCitation, BriefDeps, BriefRepo, BriefService, BriefSources, BriefStatus,
    GatewayBriefWriter, SqliteBriefRepo, StoredBrief,
};
use deino_core::calendar::SqliteCalendarRepo;
use deino_core::meetings::SqliteMeetingRepo;
use deino_domain::{CalendarEventId, FixedZone, MeetingId};
use deino_llm::{MockProvider, mock_gateway};
use deino_testkit::fixture::fixture_uuid;
use deino_testkit::{FakeClock, at, seed_fixtures, test_db};
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-12T13:30:00Z";
/// The sales discovery fixture, linked to its calendar event.
const DISCOVERY: &str = "meeting-fixture-sales-discovery";

fn discovery_event() -> CalendarEventId {
    fixture_uuid(&format!("event/{DISCOVERY}")).parse().unwrap()
}

/// An app with the `briefs` plugin over the fixtures, holding one ready
/// brief for the discovery call's event.
fn briefs_app() -> tauri::App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed_fixtures(&database).await.unwrap();
        database
    });
    let store = Arc::new(SqliteBriefRepo::new(database.clone()));
    tauri::async_runtime::block_on(store.save(&ready_brief())).unwrap();
    let gateway = Arc::new(mock_gateway(Arc::new(MockProvider::new())).gateway);
    let service = BriefService::new(BriefDeps {
        store,
        events: Arc::new(SqliteCalendarRepo::new(database.clone())),
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        sources: BriefSources::sqlite(&database, Arc::new(FixedZone::utc())),
        writer: Arc::new(GatewayBriefWriter::new(gateway)),
        clock: Arc::new(FakeClock::new(at(NOW))),
    });
    app.manage(Arc::new(service));
    app
}

fn ready_brief() -> StoredBrief {
    let meeting = BriefCitation::Meeting {
        meeting_id: MeetingId::new(DISCOVERY),
        title: "Discovery".into(),
        at: at("2026-09-30T15:01:24Z"),
    };
    StoredBrief {
        event_id: discovery_event(),
        status: BriefStatus::Ready,
        inputs_hash: "0".repeat(64),
        bullets: vec![
            BriefBullet {
                text: "Licence ends in January.".into(),
                citations: vec![meeting],
            },
            BriefBullet {
                text: "Agenda: pricing.".into(),
                citations: vec![BriefCitation::Event],
            },
        ],
        generated_at: at(NOW),
        read_at: None,
        feedback: None,
    }
}

fn call(app: &tauri::App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(&main_window(app), &format!("plugin:briefs|{command}"), args)
}

#[test]
fn for_event_returns_camel_case_bullets_with_tagged_citations() {
    let app = briefs_app();

    let brief = call(&app, "for_event", json!({ "event": discovery_event() })).unwrap();

    assert_eq!(brief["eventId"], json!(discovery_event()));
    assert_eq!(brief["generatedAt"], NOW);
    assert_eq!(brief["readAt"], Value::Null);
    assert_eq!(brief["feedback"], Value::Null);
    let citation = &brief["bullets"][0]["citations"][0];
    assert_eq!(citation["kind"], "meeting");
    assert_eq!(citation["meetingId"], DISCOVERY);
    assert_eq!(
        brief["bullets"][1]["citations"][0],
        json!({ "kind": "event" })
    );
}

#[test]
fn reading_and_feedback_are_stored_and_the_meeting_note_finds_the_brief() {
    let app = briefs_app();
    let event = json!({ "event": discovery_event() });

    assert_eq!(call(&app, "mark_read", event.clone()).unwrap(), Value::Null);
    let feedback = json!({ "event": discovery_event(), "feedback": "not_useful" });
    assert_eq!(call(&app, "give_feedback", feedback).unwrap(), Value::Null);

    let brief = call(&app, "for_meeting", json!({ "meeting": DISCOVERY })).unwrap();
    assert_eq!(brief["readAt"], NOW);
    assert_eq!(brief["feedback"], "not_useful");
}

#[test]
fn an_event_without_a_brief_is_null() {
    let app = briefs_app();
    let other = CalendarEventId::generate(&FakeClock::new(at(NOW)));

    assert_eq!(
        call(&app, "for_event", json!({ "event": other })).unwrap(),
        Value::Null
    );
}
