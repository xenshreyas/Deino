//! Contract tests: the JSON of every `export` command, through the real
//! plugin and ACL, over the seeded fixture meetings, with the dialogs,
//! clipboard, opener and file system replaced by recording fakes.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::FixedOffset;
use deino_core::export::{
    AutoExportService, ExportAssembler, ExportPorts, ExportService, ExportServiceDeps, FakeVault,
    FixedZone, FolderDialog, FullExportDeps, FullExportService, FullExportStatus,
    RecordedClipboard, RecordedOpener, SaveDialog, ScriptedDialog,
};
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_db::Database;
use deino_testkit::fixture::seed_fixtures;
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const STANDUP: &str = "meeting-fixture-standup";

/// What a test can look at after a command ran.
struct Fakes {
    vault: Arc<FakeVault>,
    clipboard: Arc<RecordedClipboard>,
    opener: Arc<RecordedOpener>,
}

/// Records full-export progress nowhere: these tests read `status`.
struct Silent;

impl deino_core::export::FullExportEvents for Silent {
    fn changed(&self, _status: &FullExportStatus) {}
}

/// The seeded fixture meetings and default settings.
fn seeded() -> (Database, Arc<SettingsService>) {
    tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed_fixtures(&database).await.unwrap();
        let store = Arc::new(FakeSettingsStore::with(None));
        let settings = SettingsService::load(store, Arc::new(RecordedSettingsEvents::default()));
        (database, Arc::new(settings.await.unwrap()))
    })
}

/// An app with the `export` plugin over the seeded fixtures; every dialog
/// answers `answer` (`None`: the user cancels).
fn export_app(answer: Option<&str>) -> (App<MockRuntime>, Fakes) {
    let app = mock_app(mock_builder().plugin(super::init()));
    let (database, settings) = seeded();
    let zone = FixedZone(FixedOffset::west_opt(7 * 3_600).unwrap());
    let assembler = Arc::new(ExportAssembler::new(ExportPorts::sqlite(
        &database,
        Arc::new(zone),
    )));
    let fakes = Fakes {
        vault: Arc::new(FakeVault::default()),
        clipboard: Arc::new(RecordedClipboard::default()),
        opener: Arc::new(RecordedOpener::default()),
    };
    let dialog = Arc::new(ScriptedDialog::answering(answer.map(PathBuf::from)));
    app.manage(Arc::new(ExportService::new(ExportServiceDeps {
        assembler: assembler.clone(),
        vault: fakes.vault.clone(),
        save_dialog: dialog.clone(),
        clipboard: fakes.clipboard.clone(),
        opener: fakes.opener.clone(),
    })));
    app.manage(Arc::new(AutoExportService::new(
        assembler.clone(),
        settings,
        fakes.vault.clone(),
    )));
    app.manage(Arc::new(FullExportService::new(FullExportDeps {
        assembler,
        lifecycle: Arc::new(SqliteMeetingRepo::new(database)),
        events: Arc::new(Silent),
        clock: Arc::new(FakeClock::new(at("2026-10-09T12:00:00Z"))),
    })));
    let folder_dialog: Arc<dyn FolderDialog> = dialog.clone();
    let save_dialog: Arc<dyn SaveDialog> = dialog;
    app.manage(folder_dialog);
    app.manage(save_dialog);
    (app, fakes)
}

fn call(app: &App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(&main_window(app), &format!("plugin:export|{command}"), args)
}

fn all_sections() -> Value {
    json!({ "notes": true, "myNotes": true, "actionItems": true, "insights": false, "transcript": true, "attendees": true })
}

#[test]
fn preview_returns_the_file_text_and_its_suggested_name() {
    let (app, _) = export_app(None);

    let preview = call(
        &app,
        "preview",
        json!({ "meetingId": STANDUP, "format": "markdown", "sections": all_sections() }),
    )
    .unwrap();

    assert_eq!(preview["format"], "markdown");
    assert_eq!(preview["kind"], "text");
    assert_eq!(preview["fileName"], "2026-10-08 Eng standup.md");
    assert!(
        preview["content"]
            .as_str()
            .unwrap()
            .starts_with("---\ntitle: \"Eng standup\"")
    );
}

#[test]
fn save_writes_where_the_dialog_points_and_names_the_file() {
    let (app, fakes) = export_app(Some("/Users/me/Desktop/standup.docx"));

    let saved = call(
        &app,
        "save",
        json!({ "meetingId": STANDUP, "format": "docx", "sections": all_sections() }),
    );

    assert_eq!(saved, Ok(json!({ "fileName": "standup.docx" })));
    assert_eq!(
        fakes.vault.paths(),
        [PathBuf::from("/Users/me/Desktop/standup.docx")]
    );
}

#[test]
fn a_cancelled_save_returns_null() {
    let (app, fakes) = export_app(None);

    let saved = call(
        &app,
        "save",
        json!({ "meetingId": STANDUP, "format": "pdf", "sections": all_sections() }),
    );

    assert_eq!(saved, Ok(Value::Null));
    assert!(fakes.vault.paths().is_empty());
}

#[test]
fn save_many_writes_every_meeting_into_the_chosen_folder() {
    let (app, fakes) = export_app(Some("/Users/me/Exports"));

    let saved = call(
        &app,
        "save_many",
        json!({ "meetingIds": [STANDUP, STANDUP], "format": "markdown" }),
    );

    assert_eq!(saved, Ok(json!({ "count": 2, "folder": "Exports" })));
    assert_eq!(
        fakes.vault.paths(),
        [
            PathBuf::from("/Users/me/Exports/2026-10-08 Eng standup (2).md"),
            PathBuf::from("/Users/me/Exports/2026-10-08 Eng standup.md"),
        ]
    );
}

#[test]
fn copy_and_email_reach_the_clipboard_and_the_mail_app() {
    let (app, fakes) = export_app(None);

    let copied = call(
        &app,
        "copy",
        json!({ "meetingId": STANDUP, "kind": "rich_text" }),
    );
    let emailed = call(&app, "email", json!({ "meetingId": STANDUP }));

    assert_eq!((copied, emailed), (Ok(Value::Null), Ok(Value::Null)));
    assert!(fakes.clipboard.copies()[0].html.is_some());
    assert!(fakes.opener.urls()[0].starts_with("mailto:"));
}

#[test]
fn choose_folder_returns_the_folder_it_saved() {
    let (app, _) = export_app(Some("/Users/me/Obsidian"));

    let folder = call(&app, "choose_folder", Value::Null);

    assert_eq!(folder, Ok(json!("/Users/me/Obsidian")));
}

#[test]
fn the_account_export_reports_an_idle_status_and_a_cancelled_dialog() {
    let (app, _) = export_app(None);

    let status = call(&app, "full_export_status", Value::Null);
    let started = call(&app, "start_full_export", json!({ "includeAudio": false }));
    let cancelled = call(&app, "cancel_full_export", Value::Null);

    assert_eq!(
        status,
        Ok(json!({ "state": "idle", "done": 0, "total": 0, "fileName": null, "message": null }))
    );
    assert_eq!((started, cancelled), (Ok(json!(false)), Ok(Value::Null)));
}

#[test]
fn errors_arrive_as_app_errors_with_a_code() {
    let (app, _) = export_app(None);

    let missing = call(
        &app,
        "copy",
        json!({ "meetingId": "meeting-x", "kind": "markdown" }),
    );

    assert_eq!(
        missing,
        Err(json!({ "code": "not_found", "message": "meeting meeting-x was not found" }))
    );
}
