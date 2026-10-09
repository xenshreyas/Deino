//! Contract tests: `search`'s JSON in and out through the real plugin and ACL.

use std::sync::Arc;

use deino_core::people::{FakePeopleRepo, PeopleService, Person, PersonRepo};
use deino_domain::{Email, PersonId};
use deino_testkit::at;
use serde_json::json;
use tauri::Manager;
use tauri::test::mock_builder;

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const PRIYA: &str = "0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e";

fn priya() -> Person {
    let created_at = at("2026-10-08T09:00:00Z");
    Person {
        id: PRIYA.parse::<PersonId>().unwrap(),
        name: "Priya Shah".into(),
        email: Some(Email::parse("priya@example.com").unwrap()),
        company_id: None,
        title: None,
        avatar_path: None,
        notes_md: String::new(),
        summary_md: None,
        summary_updated_at: None,
        is_me: false,
        merged_into: None,
        created_at,
        updated_at: created_at,
    }
}

#[test]
fn search_takes_a_query_and_returns_people_with_camel_case_fields() {
    let repo = FakePeopleRepo::default();
    tauri::async_runtime::block_on(repo.create(&priya())).unwrap();
    let app = mock_app(mock_builder().plugin(super::init()));
    app.manage(Arc::new(PeopleService::new(Arc::new(repo))));

    let found = invoke(
        &main_window(&app),
        "plugin:people|search",
        json!({ "query": "pri" }),
    )
    .unwrap();

    assert_eq!(found[0]["id"], PRIYA);
    assert_eq!(found[0]["name"], "Priya Shah");
    assert_eq!(found[0]["email"], "priya@example.com");
    assert_eq!(found[0]["mergedInto"], json!(null));
    assert_eq!(found.as_array().unwrap().len(), 1);
}
