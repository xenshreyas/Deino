//! Contract tests: `media` and `download` JSON in and out through the real
//! plugin and ACL, over a fake meeting whose recording lives in a scratch
//! recordings folder, with a scripted save dialog.

use std::sync::Arc;

use deino_core::export::ScriptedDialog;
use deino_core::meetings::{FakeMeetingRepo, Meeting, MeetingAudio, MeetingLifecycleRepo};
use deino_core::player::{FsMediaFiles, PlayerDeps, PlayerService};
use deino_domain::MeetingId;
use deino_testkit::at;
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::mock_builder;
use tempfile::TempDir;

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const MEETING: &str = "meeting-1";

/// A meeting with a microphone track on disk; the dialog saves to `saved.m4a`.
fn service(scratch: &TempDir) -> PlayerService {
    let recordings = scratch.path().join("recordings");
    let mic = recordings.join(MEETING).join("mic.m4a");
    std::fs::create_dir_all(mic.parent().unwrap()).unwrap();
    std::fs::write(&mic, b"aac").unwrap();
    let meetings = Arc::new(FakeMeetingRepo::with([Meeting {
        id: MeetingId::new(MEETING),
        title: "Standup".into(),
        created_at: at("2026-10-08T09:00:00Z"),
        updated_at: at("2026-10-08T09:00:00Z"),
    }]));
    let audio = MeetingAudio {
        audio_mic_path: Some(mic.to_string_lossy().into_owned()),
        ..MeetingAudio::default()
    };
    let at = at("2026-10-08T10:00:00Z");
    let id = MeetingId::new(MEETING);
    tauri::async_runtime::block_on(meetings.set_audio(&id, &audio, at)).unwrap();
    PlayerService::new(PlayerDeps {
        lifecycle: meetings.clone(),
        meetings,
        files: Arc::new(FsMediaFiles::new(recordings)),
        dialog: Arc::new(ScriptedDialog::answering(Some(scratch.path().join("saved.m4a")))),
    })
}

/// `plugin:player|<command>` with `args`.
fn call(scratch: &TempDir, command: &str, args: &Value) -> Result<Value, Value> {
    let app = mock_app(mock_builder().plugin(super::init()));
    app.manage(Arc::new(service(scratch)));
    invoke(&main_window(&app), &format!("plugin:player|{command}"), args.clone())
}

#[test]
fn media_takes_a_meeting_id_and_lists_files_by_camel_case_source() {
    let scratch = tempfile::tempdir().unwrap();

    let media = call(&scratch, "media", &json!({ "meetingId": MEETING })).unwrap();

    assert_eq!(media["files"][0]["source"], "microphone");
    let path = media["files"][0]["path"].as_str().unwrap();
    assert!(path.ends_with("mic.m4a"), "{path}");
    assert_eq!(media["files"].as_array().unwrap().len(), 1);
}

#[test]
fn download_takes_a_source_and_answers_saved() {
    let scratch = tempfile::tempdir().unwrap();

    let outcome = call(
        &scratch,
        "download",
        &json!({ "meetingId": MEETING, "source": "microphone" }),
    );

    assert_eq!(outcome, Ok(json!("saved")));
    assert!(scratch.path().join("saved.m4a").exists());
}

#[test]
fn downloading_a_missing_source_is_not_found() {
    let scratch = tempfile::tempdir().unwrap();

    let error = call(
        &scratch,
        "download",
        &json!({ "meetingId": MEETING, "source": "video" }),
    )
    .unwrap_err();

    assert_eq!(error["code"], "not_found");
}
