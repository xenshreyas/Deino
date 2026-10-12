//! Contract tests: the JSON the UI sends to and receives from each stats
//! command, through the real plugin, ACL and error mapping.

use std::sync::Arc;

use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_core::stats::{
    RecapParts, RecapService, RecapWindow, SqliteRecapRepo, StatsPorts, StatsService,
};
use deino_db::test_db;
use deino_domain::FixedZone;
use deino_testkit::{FakeClock, at, seed};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn window() -> WebviewWindow<MockRuntime> {
    let database = block_on(test_db());
    block_on(seed::meeting(
        &database,
        "meeting-1",
        "Kickoff",
        at("2026-10-05T09:30:00Z"),
    ));
    let settings = block_on(SettingsService::load(
        Arc::new(FakeSettingsStore::with(None)),
        Arc::new(RecordedSettingsEvents::default()),
    ))
    .unwrap();
    let ports = StatsPorts::sqlite(
        &database,
        Arc::new(settings),
        Arc::new(FixedZone::utc()),
        Arc::new(FakeClock::new(at("2026-10-11T12:00:00Z"))),
    );
    let recap = RecapService::new(
        ports.clone(),
        RecapParts {
            recaps: Arc::new(SqliteRecapRepo::new(database)),
            gateway: None,
            window: RecapWindow::December,
        },
    );
    let builder = mock_builder()
        .plugin(super::init())
        .manage(Arc::new(StatsService::new(ports)))
        .manage(Arc::new(recap));
    main_window(&mock_app(builder))
}

#[test]
fn overview_takes_a_range_and_returns_the_numbers() {
    let window = window();

    let stats = invoke(
        &window,
        "plugin:stats|overview",
        json!({ "range": "last_30_days" }),
    )
    .unwrap();

    assert_eq!(stats["since"], "2026-09-12T00:00:00Z");
    assert_eq!(stats["meetings"], 1);
    assert_eq!(stats["recordedSeconds"], 0);
    assert_eq!(stats["people"], 0);
    assert_eq!(
        stats["audience"],
        json!({ "internal": 0, "external": 0, "unknown": 1 })
    );
    assert_eq!(stats["topPeople"], json!([]));
    assert_eq!(stats["weekdays"], json!([1, 0, 0, 0, 0, 0, 0]));
    assert_eq!(stats["hours"][9], 1);
    assert_eq!(
        stats["llm"],
        json!({ "models": [], "estimatedCostMicros": 0, "unpricedCalls": 0 })
    );
}

#[test]
fn heatmap_returns_local_days() {
    let heatmap = invoke(&window(), "plugin:stats|heatmap", Value::Null).unwrap();

    assert_eq!(
        heatmap,
        json!({
            "firstDay": "2025-10-06",
            "lastDay": "2026-10-11",
            "days": [{ "day": "2026-10-05", "meetings": 1 }],
        })
    );
}

#[test]
fn the_recap_is_closed_outside_december() {
    let window = window();

    let status = invoke(&window, "plugin:stats|recap", Value::Null).unwrap();
    let error = invoke(&window, "plugin:stats|write_recap", Value::Null);

    assert_eq!(
        status,
        json!({ "year": 2026, "available": false, "recap": null })
    );
    assert_eq!(
        error,
        Err(json!({ "code": "conflict", "message": "the year in review opens in December" }))
    );
}
