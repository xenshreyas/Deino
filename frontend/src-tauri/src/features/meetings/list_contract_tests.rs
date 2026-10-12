//! Contract tests of ORG-07's commands: the JSON the Meetings page sends
//! and receives for the list, views, trash, bulk actions and merge, through
//! the real plugin, ACL and error mapping over an in-memory database.

use std::sync::Arc;

use deino_core::folders::{FilingService, FolderPorts, SqliteFolderRepo};
use deino_core::meetings::{
    DiskMeetingFiles, MeetingBulkService, MeetingListDeps, MeetingListService, MeetingMergeService,
    MeetingViewService, MergeDeps, SqliteMeetingRepo, TrashDeps, TrashService,
};
use deino_core::notes::SqliteNotesRepo;
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_core::tags::{RecordedTagEvents, SqliteTagRepo, TagPorts, TagService};
use deino_db::Database;
use deino_domain::{Clock, FixedZone};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// Three finished meetings, a day apart: `m0` (oldest) to `m2`.
fn window() -> WebviewWindow<MockRuntime> {
    let database = seeded();
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
    let repo = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let merge = MeetingMergeService::new(MergeDeps {
        repo: repo.clone(),
        notes: Arc::new(SqliteNotesRepo::new(database.clone())),
        clock: Arc::clone(&clock),
    });
    let builder = mock_builder()
        .plugin(super::init())
        .manage(Arc::new(list(repo.clone())))
        .manage(Arc::new(trash(repo.clone(), &clock)))
        .manage(Arc::new(bulk(&database, &clock)))
        .manage(Arc::new(MeetingViewService::new(repo, clock)))
        .manage(Arc::new(merge));
    main_window(&mock_app(builder))
}

fn seeded() -> Database {
    let database = block_on(test_db());
    for (n, day) in ["2026-10-01", "2026-10-02", "2026-10-03"]
        .iter()
        .enumerate()
    {
        let created_at = at(&format!("{day}T09:00:00Z"));
        block_on(seed::meeting(
            &database,
            &format!("m{n}"),
            "Standup",
            created_at,
        ));
    }
    database
}

/// The list, with default settings (no address of the user's).
fn list(repo: Arc<SqliteMeetingRepo>) -> MeetingListService {
    let store = Arc::new(FakeSettingsStore::default());
    let events = Arc::new(RecordedSettingsEvents::default());
    let settings = Arc::new(block_on(SettingsService::load(store, events)).unwrap());
    MeetingListService::new(MeetingListDeps {
        repo,
        zone: Arc::new(FixedZone::utc()),
        addresses: settings,
    })
}

fn trash(repo: Arc<SqliteMeetingRepo>, clock: &Arc<dyn Clock>) -> TrashService {
    let data = std::env::temp_dir().join("deino-org07-contract");
    TrashService::new(TrashDeps {
        bin: repo,
        files: Arc::new(DiskMeetingFiles::new(
            data.join("recordings"),
            data.join("meetings"),
        )),
        clock: Arc::clone(clock),
    })
}

fn bulk(database: &Database, clock: &Arc<dyn Clock>) -> MeetingBulkService {
    let tag_repo = Arc::new(SqliteTagRepo::new(database.clone()));
    let tags = Arc::new(TagService::new(TagPorts {
        tags: tag_repo.clone(),
        applied: tag_repo,
        events: Arc::new(RecordedTagEvents::default()),
        clock: Arc::clone(clock),
    }));
    let folders = FolderPorts::over(
        Arc::new(SqliteFolderRepo::new(database.clone())),
        Arc::clone(clock),
    );
    MeetingBulkService::new(tags, Arc::new(FilingService::new(&folders)))
}

fn ids(page: &Value) -> Vec<&str> {
    page["meetings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect()
}

#[test]
fn a_page_takes_a_typed_filter_and_returns_rows_and_a_cursor() {
    let window = window();
    let filter = json!({ "dateRange": { "from": "2026-10-02", "to": null }, "tags": [] });

    let page = invoke(
        &window,
        "plugin:meetings|page",
        json!({ "filter": filter, "after": null, "size": 1 }),
    )
    .unwrap();
    let rest = invoke(
        &window,
        "plugin:meetings|page",
        json!({ "filter": filter, "after": page["next"], "size": 5 }),
    )
    .unwrap();

    assert_eq!(ids(&page), ["m2"]);
    assert_eq!(page["next"]["id"], "m2");
    assert_eq!(page["meetings"][0]["attendees"], json!([]));
    assert_eq!(page["meetings"][0]["status"], "ready");
    assert_eq!(ids(&rest), ["m1"]);
    assert_eq!(rest["next"], Value::Null);
}

#[test]
fn the_audience_filter_without_an_address_is_invalid() {
    let filter = json!({ "audience": "external" });

    let error = invoke(
        &window(),
        "plugin:meetings|matching_ids",
        json!({ "filter": filter }),
    )
    .unwrap_err();

    assert_eq!(error["code"], "invalid");
}

#[test]
fn views_round_trip_with_their_filter() {
    let window = window();
    let view = json!({ "name": "With items", "filter": { "hasActionItems": true } });

    let created = invoke(
        &window,
        "plugin:meetings|create_view",
        json!({ "view": view }),
    )
    .unwrap();
    let listed = invoke(&window, "plugin:meetings|views", Value::Null).unwrap();
    let duplicate = invoke(
        &window,
        "plugin:meetings|create_view",
        json!({ "view": view }),
    )
    .unwrap_err();

    assert_eq!(listed[0]["id"], created["id"]);
    assert_eq!(listed[0]["filter"]["hasActionItems"], true);
    assert_eq!(listed[0]["filter"]["people"], json!([]));
    assert_eq!(duplicate["code"], "conflict");
}

#[test]
fn trash_restore_and_delete_forever_report_what_changed() {
    let window = window();
    let some = json!({ "ids": ["m0", "m1"] });

    let trashed = invoke(&window, "plugin:meetings|trash", some.clone()).unwrap();
    let bin = invoke(&window, "plugin:meetings|trashed", Value::Null).unwrap();
    let restored = invoke(&window, "plugin:meetings|restore", json!({ "ids": ["m1"] })).unwrap();
    let deleted = invoke(&window, "plugin:meetings|delete_forever", some).unwrap();

    assert_eq!(trashed, json!({ "changed": ["m0", "m1"] }));
    assert_eq!(bin[0]["purgesAt"], "2026-11-11T09:00:00Z");
    assert_eq!(restored, json!({ "changed": ["m1"] }));
    assert_eq!(deleted, json!({ "changed": ["m0"] }), "m1 left the trash");
}

#[test]
fn merging_returns_the_surviving_meeting_and_tagging_an_unknown_tag_fails() {
    let window = window();

    let merged = invoke(
        &window,
        "plugin:meetings|merge",
        json!({ "ids": ["m2", "m1"] }),
    )
    .unwrap();
    let refused = invoke(&window, "plugin:meetings|merge", json!({ "ids": ["m0"] })).unwrap_err();
    let tag = json!({ "ids": ["m0"], "tag": "00000000-0000-7000-8000-000000000001" });
    let missing = invoke(&window, "plugin:meetings|tag_many", tag).unwrap_err();

    assert_eq!(merged, json!({ "meetingId": "m1", "absorbed": ["m2"] }));
    assert_eq!(refused["code"], "invalid");
    assert_eq!(missing["code"], "not_found");
}
