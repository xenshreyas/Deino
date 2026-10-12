//! Contract tests: the JSON of the retention commands, through the real
//! plugin, ACL and error mapping, over a meeting whose transcript retention
//! deleted.

use std::sync::Arc;

use deino_core::meetings::trash::StoredFiles;
use deino_core::meetings::{MeetingError, NoMeetingWork};
use deino_core::retention::{
    MeetingTrash, RecordingFiles, RetentionDeps, RetentionPolicy, RetentionService,
    SqliteRetentionStore,
};
use deino_core::settings::RetentionSettings;
use deino_domain::MeetingId;
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-12T13:30:00Z";

/// Keeps everything; the sweep never runs in these tests.
struct KeepAll;

#[async_trait::async_trait]
impl RetentionPolicy for KeepAll {
    async fn retention(&self) -> RetentionSettings {
        RetentionSettings::default()
    }
}

#[async_trait::async_trait]
impl RecordingFiles for KeepAll {
    async fn delete_recordings(&self, _files: &StoredFiles) -> std::io::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl MeetingTrash for KeepAll {
    async fn trash(&self, _ids: &[MeetingId]) -> Result<Vec<MeetingId>, MeetingError> {
        Ok(Vec::new())
    }
}

/// An app with the `retention` plugin over one meeting whose transcript
/// was deleted and one untouched meeting.
fn retention_app() -> tauri::App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed::meeting(&database, "gone", "Old", at("2026-09-01T09:00:00Z")).await;
        seed::meeting(&database, "kept", "New", at("2026-10-10T09:00:00Z")).await;
        sqlx::query("UPDATE meetings SET transcript_deleted_at = ?1 WHERE id = 'gone'")
            .bind(at("2026-10-01T09:00:00Z"))
            .execute(database.writer())
            .await
            .unwrap();
        database
    });
    let service = RetentionService::new(RetentionDeps {
        store: Arc::new(SqliteRetentionStore::new(database)),
        policy: Arc::new(KeepAll),
        files: Arc::new(KeepAll),
        trash: Arc::new(KeepAll),
        work: Arc::new(NoMeetingWork),
        clock: Arc::new(FakeClock::new(at(NOW))),
    });
    app.manage(Arc::new(service));
    app
}

fn call(app: &tauri::App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(
        &main_window(app),
        &format!("plugin:retention|{command}"),
        args,
    )
}

#[test]
fn status_says_when_the_transcript_and_recording_were_deleted() {
    let app = retention_app();

    let gone = call(&app, "status", json!({ "meeting": "gone" })).unwrap();
    let kept = call(&app, "status", json!({ "meeting": "kept" })).unwrap();

    assert_eq!(
        gone,
        json!({ "audioDeletedAt": null, "transcriptDeletedAt": "2026-10-01T09:00:00Z" })
    );
    assert_eq!(
        kept,
        json!({ "audioDeletedAt": null, "transcriptDeletedAt": null })
    );
}

#[test]
fn a_missing_meeting_is_not_found() {
    let app = retention_app();

    let missing = call(&app, "status", json!({ "meeting": "missing" }));

    assert_eq!(missing.unwrap_err()["code"], "not_found");
}
