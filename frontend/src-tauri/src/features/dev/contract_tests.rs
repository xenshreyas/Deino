//! Contract tests of the `dev` plugin through the real plugin and ACL.

use std::sync::Arc;

use deino_testkit::fixture::{FixtureSeeder, Fixtures};
use deino_testkit::test_db;
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::mock_builder;

use crate::features::ipc_tests::{invoke, main_window, mock_app};

#[test]
fn seed_fixtures_reports_what_it_wrote_and_the_meetings_appear() {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(test_db());
    app.manage(Arc::new(FixtureSeeder::new(database.clone())));
    let fixtures = Fixtures::load().unwrap();
    let segments: usize = fixtures.meetings().iter().map(|m| m.segments.len()).sum();

    let report = invoke(&main_window(&app), "plugin:dev|seed_fixtures", Value::Null);

    let expected = json!({ "meetings": fixtures.meetings().len(), "segments": segments });
    assert_eq!(report, Ok(expected));
    let seeded: i64 = tauri::async_runtime::block_on(
        sqlx::query_scalar("SELECT COUNT(*) FROM meetings WHERE id LIKE 'meeting-fixture-%'")
            .fetch_one(database.reader()),
    )
    .unwrap();
    assert_eq!(usize::try_from(seeded).unwrap(), fixtures.meetings().len());
}
