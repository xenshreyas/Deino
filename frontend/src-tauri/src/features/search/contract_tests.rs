//! Contract tests: the JSON of the search commands through the real plugin,
//! ACL and error mapping, over SQLite.

use std::sync::Arc;

use deino_core::models::{
    Catalog, FakeByteSource, FakeServer, ModelPurpose, ModelService, ModelServiceParts, ModelStore,
    RecordedModelEvents,
};
use deino_core::search::semantic::{
    AdminParts, Backfill, IndexerParts, NoEmbedder, NoSemanticSearch, SemanticAdmin,
    SemanticIndexer, SharedVectors, SqliteEmbeddingStore,
};
use deino_core::search::{SearchService, SqliteSearchIndex};
use deino_db::Database;
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewWindow};

use super::TauriIndexEvents;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-09T10:00:00Z";

fn database() -> Database {
    tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed::meeting(&database, "meeting-1", "Roadmap review", at(NOW)).await;
        sqlx::query(
            "INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text)
             VALUES ('0192f3a0-0000-7000-8000-0000000000a1', 'meeting-1', 'mixed', 61000, 64000,
                     'the calendar beta ships in November')",
        )
        .execute(database.writer())
        .await
        .unwrap();
        database
    })
}

/// The admin over an empty models folder and a store with nothing embedded.
fn admin(app: &App<MockRuntime>, database: &Database, dir: &std::path::Path) -> SemanticAdmin {
    let models = Arc::new(
        ModelService::new(ModelServiceParts {
            catalog: Catalog::bundled().unwrap(),
            store: ModelStore::new(dir),
            source: Arc::new(FakeByteSource::new(Vec::new(), FakeServer::Ranged)),
            events: Arc::new(RecordedModelEvents::default()),
            memory_bytes: 0,
        })
        .serving(ModelPurpose::Embedding),
    );
    let store = Arc::new(SqliteEmbeddingStore::new(database));
    let indexer = Arc::new(SemanticIndexer::new(IndexerParts {
        sources: store.clone(),
        store: store.clone(),
        embedder: Arc::new(NoEmbedder),
        vectors: Arc::new(SharedVectors::new(store.clone(), "nomic-embed-text-v1.5")),
        clock: Arc::new(FakeClock::new(at(NOW))),
    }));
    let events = Arc::new(TauriIndexEvents::new(app.handle().clone()));
    SemanticAdmin::new(AdminParts {
        models,
        store: store.clone(),
        backfill: Arc::new(Backfill::new(indexer, store, events)),
        model: "nomic-embed-text-v1.5".into(),
    })
}

fn search_app(dir: &std::path::Path) -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = database();
    let index = Arc::new(SqliteSearchIndex::new(&database));
    app.manage(Arc::new(SearchService::new(
        index.clone(),
        index,
        Arc::new(NoSemanticSearch),
    )));
    let admin = admin(&app, &database, dir);
    app.manage(Arc::new(admin));
    app
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:search|{command}"), args)
}

#[test]
fn search_returns_sections_of_hits_with_highlight_ranges_in_camel_case() {
    let dir = tempfile::tempdir().unwrap();
    let app = search_app(dir.path());
    let request =
        json!({ "text": "calendar", "filters": { "chips": [], "domains": [] }, "limit": 5 });

    let results = call(&main_window(&app), "search", json!({ "request": request })).unwrap();

    let moment = &results["moments"][0];
    assert_eq!(moment["kind"], "segment");
    assert_eq!(moment["meetingId"], "meeting-1");
    assert_eq!(moment["refId"], "0192f3a0-0000-7000-8000-0000000000a1");
    assert_eq!(moment["tsMs"], 61_000);
    assert_eq!(moment["endMs"], json!(null));
    assert_eq!(moment["label"]["text"], "Roadmap review");
    assert_eq!(
        moment["snippet"]["highlights"][0],
        json!({ "start": 4, "end": 12 })
    );
    assert!(moment["score"].as_f64().unwrap() > 0.0);
    assert_eq!(results["meetings"][0]["meetingId"], "meeting-1");
    assert_eq!(results["actionItems"], json!([]));
    assert_eq!(results["semantic"], false);
}

#[test]
fn a_request_may_omit_its_filters() {
    let dir = tempfile::tempdir().unwrap();
    let app = search_app(dir.path());

    let results = call(
        &main_window(&app),
        "search",
        json!({ "request": { "text": "roadmap", "limit": 5 } }),
    )
    .unwrap();

    assert_eq!(results["filters"], json!({ "chips": [], "domains": [] }));
    assert_eq!(results["meetings"][0]["kind"], "meeting");
}

#[test]
fn filter_suggestions_are_chips_and_reindex_reports_the_documents() {
    let dir = tempfile::tempdir().unwrap();
    let app = search_app(dir.path());
    let window = main_window(&app);

    let chips = call(
        &window,
        "suggest_filters",
        json!({ "prefix": "road", "kind": "meeting" }),
    )
    .unwrap();
    let report = call(&window, "reindex_all", json!({})).unwrap();

    assert_eq!(
        chips,
        json!([{ "kind": "meeting", "id": "meeting-1", "label": "Roadmap review" }])
    );
    assert_eq!(report, json!({ "documents": 2 }));
}

#[test]
fn the_index_status_names_the_missing_model_and_the_coverage() {
    let dir = tempfile::tempdir().unwrap();
    let app = search_app(dir.path());
    let window = main_window(&app);

    let status = call(&window, "index_status", json!({})).unwrap();
    let backfill = call(&window, "start_backfill", json!({})).unwrap_err();

    assert_eq!(status["model"]["id"], "nomic-embed-text-v1.5.Q8_0");
    assert_eq!(status["model"]["state"], json!({ "kind": "missing" }));
    assert_eq!(status["model"]["sizeBytes"], 146_146_432);
    assert_eq!(
        (
            status["totalMeetings"].clone(),
            status["indexedMeetings"].clone()
        ),
        (json!(1), json!(0))
    );
    assert_eq!(
        status["backfill"],
        json!({ "running": false, "done": 0, "total": 0, "failed": 0 })
    );
    assert_eq!(backfill["code"], "unavailable");
    assert_eq!(call(&window, "cancel_download", json!({})).unwrap(), false);
}
