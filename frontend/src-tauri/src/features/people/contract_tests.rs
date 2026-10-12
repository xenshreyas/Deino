//! Contract tests: the JSON of the `people` commands through the real
//! plugin, ACL and error mapping, over SQLite.

use std::sync::Arc;

use deino_core::entities::{AttendanceService, EntityPorts, EntitySummarizer, PersonService};
use deino_core::people::{PeopleService, SqlitePeopleRepo};
use deino_domain::Clock;
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewWindow};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-11T12:00:00Z";

/// An app with the `people` plugin over a database holding `meeting-1`, and
/// no LLM (summaries answer `no_model`).
fn people_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed::meeting(&database, "meeting-1", "Standup", at(NOW)).await;
        database
    });
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at(NOW)));
    let ports = EntityPorts::sqlite(&database, clock);
    let search = PeopleService::new(Arc::new(SqlitePeopleRepo::new(database)));
    app.manage(Arc::new(search));
    app.manage(Arc::new(PersonService::new(ports.clone())));
    app.manage(Arc::new(AttendanceService::new(ports.clone())));
    app.manage(Arc::new(EntitySummarizer::new(ports, None)));
    app
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:people|{command}"), args)
}

fn create(window: &WebviewWindow<MockRuntime>, name: &str, email: &Value) -> Value {
    let fields = json!({ "name": name, "email": email, "title": null, "companyId": null });
    call(window, "create", json!({ "fields": fields })).unwrap()
}

#[test]
fn creating_returns_a_camel_case_person_and_a_taken_address_is_a_conflict() {
    let app = people_app();
    let window = main_window(&app);

    let owen = create(&window, "Owen Price", &json!("owen@fernhollow.co"));
    let fields =
        json!({ "name": "Owen", "email": "owen@fernhollow.co", "title": null, "companyId": null });
    let clash = call(&window, "create", json!({ "fields": fields })).unwrap_err();

    assert_eq!(owen["name"], "Owen Price");
    assert_eq!(owen["email"], "owen@fernhollow.co");
    assert!(owen["companyId"].is_string(), "{owen}");
    assert_eq!(owen["mergedInto"], json!(null));
    assert_eq!(clash["code"], "conflict");
}

#[test]
fn the_list_and_page_have_the_shapes_the_ui_reads() {
    let app = people_app();
    let window = main_window(&app);
    let maya = create(&window, "Maya Chen", &json!(null));
    call(
        &window,
        "add_attendee",
        json!({ "meetingId": "meeting-1", "personId": maya["id"] }),
    )
    .unwrap();

    let list = call(&window, "list", json!({})).unwrap();
    let page = call(&window, "page", json!({ "id": maya["id"] })).unwrap();
    let attendees = call(&window, "attendees", json!({ "meetingId": "meeting-1" })).unwrap();

    assert_eq!(list[0]["meetingCount"], 1);
    assert!(list[0]["lastMetAt"].is_string());
    assert_eq!(page["person"]["name"], "Maya Chen");
    assert_eq!(page["meetings"][0]["title"], "Standup");
    assert_eq!(page["actionItems"], json!([]));
    assert_eq!(page["upcoming"], json!([]));
    assert_eq!(attendees[0]["role"], "invitee");
    assert_eq!(attendees[0]["isMe"], false);
}

#[test]
fn merging_notes_mentions_and_summaries_answer_through_ipc() {
    let app = people_app();
    let window = main_window(&app);
    let kept = create(&window, "Priya Nair", &json!(null));
    let gone = create(&window, "Priya", &json!("priya@deino.dev"));

    let merged = call(
        &window,
        "merge",
        json!({ "survivor": kept["id"], "absorbed": gone["id"] }),
    )
    .unwrap();
    let same = call(
        &window,
        "merge",
        json!({ "survivor": kept["id"], "absorbed": kept["id"] }),
    )
    .unwrap_err();
    call(
        &window,
        "set_notes",
        json!({ "id": kept["id"], "notes": "Owns infra." }),
    )
    .unwrap();
    let mentioned = call(&window, "from_mention", json!({ "name": "priya nair" })).unwrap();
    let refreshed = call(&window, "refresh_summary", json!({ "id": kept["id"] })).unwrap();
    let removed = call(
        &window,
        "remove_attendee",
        json!({ "meetingId": "meeting-1", "personId": kept["id"] }),
    )
    .unwrap();

    assert_eq!(merged["email"], "priya@deino.dev");
    assert_eq!(same["code"], "invalid");
    assert_eq!(mentioned["id"], kept["id"]);
    assert_eq!(mentioned["notesMd"], "Owns infra.");
    assert_eq!(refreshed, "no_model");
    assert_eq!(removed, false);
}

#[test]
fn search_takes_a_query_and_returns_people() {
    let app = people_app();
    let window = main_window(&app);
    create(&window, "Priya Shah", &json!("priya@example.com"));

    let found = call(&window, "search", json!({ "query": "pri" })).unwrap();

    assert_eq!(found[0]["name"], "Priya Shah");
    assert_eq!(found.as_array().unwrap().len(), 1);
}
