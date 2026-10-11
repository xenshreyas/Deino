//! Contract tests: the JSON the UI sends to and receives from each
//! `action-items` command, through the real plugin, ACL and error mapping.

use std::sync::Arc;

use deino_core::action_items::{
    ActionItemDeps, ActionItemService, OptionSources, RecordedActionItemEvents, SavedViewService,
    SqliteActionItemRepo,
};
use deino_core::companies::SqliteCompanyRepo;
use deino_core::folders::SqliteFolderRepo;
use deino_core::people::{MeRepo, Person, PersonRepo, SqlitePeopleRepo};
use deino_core::tags::SqliteTagRepo;
use deino_db::Database;
use deino_domain::{Email, FixedZone, PersonId};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// "Now": Friday 9 October 2026, 09:00 in California.
const NOW: &str = "2026-10-09T16:00:00Z";

/// A database with the standup and the user, Shreyas; returns the window
/// and the user's ID.
fn window() -> (WebviewWindow<MockRuntime>, PersonId) {
    let database = block_on(test_db());
    block_on(seed::meeting(
        &database,
        "meeting-standup",
        "Eng standup",
        at(NOW),
    ));
    let me = block_on(user(&database));
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let repo = Arc::new(SqliteActionItemRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let items = ActionItemService::new(ActionItemDeps {
        items: repo.clone(),
        batch: repo.clone(),
        reader: repo.clone(),
        me: people.clone(),
        options: OptionSources {
            people,
            companies: Arc::new(SqliteCompanyRepo::new(database.clone())),
            tags: Arc::new(SqliteTagRepo::new(database.clone())),
            folders: Arc::new(SqliteFolderRepo::new(database.clone())),
        },
        events: Arc::new(RecordedActionItemEvents::default()),
        clock: clock.clone(),
        zone: Arc::new(FixedZone::hours(-7).unwrap()),
    });
    let builder = mock_builder()
        .plugin(super::init())
        .manage(Arc::new(items))
        .manage(Arc::new(SavedViewService::new(repo, clock)));
    (main_window(&mock_app(builder)), me)
}

/// Shreyas, made the user's "Me" person.
async fn user(database: &Database) -> PersonId {
    let people = SqlitePeopleRepo::new(database.clone());
    let me = Person {
        id: PersonId::generate(&FakeClock::new(at(NOW))),
        name: "Shreyas Rao".into(),
        email: Some(Email::parse("shreyas@deino.dev").unwrap()),
        company_id: None,
        title: None,
        avatar_path: None,
        notes_md: String::new(),
        summary_md: None,
        summary_updated_at: None,
        is_me: false,
        merged_into: None,
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    people.create(&me).await.unwrap();
    people.make_me(&me.id, at(NOW)).await.unwrap();
    me.id
}

fn create(window: &WebviewWindow<MockRuntime>, item: &Value) -> Value {
    invoke(
        window,
        "plugin:action-items|create",
        json!({ "item": item }),
    )
    .unwrap()
}

fn mine() -> Value {
    json!({
        "assignee": { "kind": "me" }, "status": "open", "meetingId": null, "tagId": null,
        "companyId": null, "folderId": null, "due": null, "dateRange": null
    })
}

#[test]
fn create_takes_a_camel_case_item_and_returns_its_list_row() {
    let (window, me) = window();

    let row = create(
        &window,
        &json!({ "title": "Send the deck", "description": "", "meetingId": "meeting-standup",
                "assigneePersonId": me.to_string(), "due": "2026-10-12", "priority": "high" }),
    );

    assert_eq!(row["title"], "Send the deck");
    assert_eq!(row["meetingTitle"], "Eng standup");
    assert_eq!(row["assigneeName"], "Shreyas Rao");
    assert_eq!(row["assigneeIsMe"], true);
    assert_eq!(row["status"], "open");
    assert_eq!(row["createdBy"], "user");
    assert_eq!(row["priority"], "high");
    // Due days are noon, local time (UTC-7 here).
    assert!(
        row["dueAt"]
            .as_str()
            .unwrap()
            .starts_with("2026-10-12T19:00:00")
    );
    assert_eq!(row["sourceStart"], Value::Null);
}

#[test]
fn list_takes_a_filter_and_today_splits_overdue_from_due_today() {
    let (window, me) = window();
    for (title, due) in [
        ("Late", "2026-10-08"),
        ("Now", "2026-10-09"),
        ("Later", "2026-10-20"),
    ] {
        create(
            &window,
            &json!({ "title": title, "assigneePersonId": me.to_string(), "due": due }),
        );
    }

    let listed = invoke(
        &window,
        "plugin:action-items|list",
        json!({ "filter": mine() }),
    )
    .unwrap();
    let mut overdue = mine();
    overdue["due"] = json!("overdue");
    let late = invoke(
        &window,
        "plugin:action-items|list",
        json!({ "filter": overdue }),
    )
    .unwrap();
    let today = invoke(&window, "plugin:action-items|today", Value::Null).unwrap();

    let titles = |rows: &Value| -> Vec<String> {
        rows.as_array()
            .unwrap()
            .iter()
            .map(|row| row["title"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(titles(&listed), ["Late", "Now", "Later"]);
    assert_eq!(titles(&late), ["Late"]);
    assert_eq!(titles(&today["overdue"]), ["Late"]);
    assert_eq!(titles(&today["dueToday"]), ["Now"]);
}

#[test]
fn update_takes_one_tagged_edit_and_bulk_update_counts() {
    let (window, _) = window();
    let first = create(&window, &json!({ "title": "One" }));
    let second = create(&window, &json!({ "title": "Two" }));

    let done = invoke(
        &window,
        "plugin:action-items|update",
        json!({ "id": first["id"], "edit": { "field": "status", "status": "done" } }),
    )
    .unwrap();
    let count = invoke(
        &window,
        "plugin:action-items|bulk_update",
        json!({ "ids": [first["id"], second["id"]], "edit": { "field": "due", "date": null } }),
    )
    .unwrap();
    let wording = invoke(
        &window,
        "plugin:action-items|bulk_update",
        json!({ "ids": [first["id"]], "edit": { "field": "title", "title": "x" } }),
    );

    assert_eq!(done["status"], "done");
    assert!(done["completedAt"].is_string());
    assert_eq!(count, json!(2));
    assert_eq!(wording.unwrap_err()["code"], "invalid");
}

#[test]
fn delete_and_mark_all_mine_done_return_counts() {
    let (window, me) = window();
    let mine = create(
        &window,
        &json!({ "title": "Mine", "assigneePersonId": me.to_string() }),
    );
    let other = create(&window, &json!({ "title": "Nobody's" }));

    let done = invoke(
        &window,
        "plugin:action-items|mark_all_mine_done",
        Value::Null,
    )
    .unwrap();
    let deleted = invoke(
        &window,
        "plugin:action-items|delete",
        json!({ "ids": [other["id"], mine["id"]] }),
    )
    .unwrap();
    let missing = invoke(
        &window,
        "plugin:action-items|update",
        json!({ "id": mine["id"], "edit": { "field": "priority", "priority": null } }),
    );

    assert_eq!((done, deleted), (json!(1), json!(2)));
    assert_eq!(missing.unwrap_err()["code"], "not_found");
}

#[test]
fn filter_options_list_people_with_me_first() {
    let (window, _) = window();

    let options = invoke(&window, "plugin:action-items|filter_options", Value::Null).unwrap();

    assert_eq!(options["people"][0]["name"], "Shreyas Rao");
    assert_eq!(options["people"][0]["isMe"], true);
    assert_eq!(options["people"][0]["email"], "shreyas@deino.dev");
    assert_eq!(options["companies"], json!([]));
    assert_eq!(options["tags"], json!([]));
    assert_eq!(options["folders"], json!([]));
}

#[test]
fn views_are_created_renamed_listed_and_deleted() {
    let (window, _) = window();
    let view = json!({ "name": "Overdue", "filter": mine() });

    let created = invoke(
        &window,
        "plugin:action-items|create_view",
        json!({ "view": view }),
    )
    .unwrap();
    let clash = invoke(
        &window,
        "plugin:action-items|create_view",
        json!({ "view": view }),
    );
    let renamed = invoke(
        &window,
        "plugin:action-items|update_view",
        json!({ "id": created["id"], "view": { "name": "Late", "filter": mine() } }),
    )
    .unwrap();
    let listed = invoke(&window, "plugin:action-items|views", Value::Null).unwrap();
    let deleted = invoke(
        &window,
        "plugin:action-items|delete_view",
        json!({ "id": created["id"] }),
    );

    assert_eq!(created["position"], 0);
    assert_eq!(created["filter"]["assignee"]["kind"], "me");
    assert_eq!(clash.unwrap_err()["code"], "conflict");
    assert_eq!(renamed["name"], "Late");
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(deleted, Ok(Value::Null));
}
