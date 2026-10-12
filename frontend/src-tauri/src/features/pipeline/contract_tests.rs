//! Contract tests: the JSON of `status`, `rerun` and `PipelineProgress`,
//! through the real plugin and ACL, over the real registry with recorded
//! AI services in place of the gateway-backed ones.

use std::sync::{Arc, mpsc};
use std::time::Duration;

use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::segments::SqliteSegmentRepo;
use deino_pipeline::{
    FakeSleeper, PipelineDeps, PipelineService, SqliteJobRepo, StagePorts, registry,
};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Listener, Manager};

use super::TauriPipelineProgress;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);
const NOW: &str = "2026-10-09T10:00:00Z";

/// An app with the `pipeline` plugin over a database holding one ready
/// meeting (seeded the legacy way, so `ready`).
fn pipeline_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed::meeting(&database, "meeting-1", "Standup", at(NOW)).await;
        database
    });
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let ports = StagePorts {
        segments: Arc::new(SqliteSegmentRepo::new(database.clone())),
        meetings: meetings.clone(),
        ..StagePorts::fakes()
    };
    let service = Arc::new(PipelineService::new(PipelineDeps {
        jobs: Arc::new(SqliteJobRepo::new(database)),
        stages: registry::stages(&ports),
        lifecycle: Arc::new(MeetingLifecycleService::new(
            meetings.clone(),
            clock.clone(),
        )),
        meetings,
        clock: clock.clone(),
        sleeper: Arc::new(FakeSleeper::new(clock)),
        progress: Arc::new(TauriPipelineProgress::new(app.handle().clone())),
    }));
    // Not started: the runner stays closed, so no stage runs and every
    // response is exactly what the command itself wrote.
    app.manage(service);
    app
}

#[test]
fn status_is_empty_for_a_meeting_the_pipeline_never_ran_for() {
    let app = pipeline_app();

    let status = invoke(
        &main_window(&app),
        "plugin:pipeline|status",
        json!({ "meetingId": "meeting-1" }),
    );

    assert_eq!(status, Ok(json!([])));
}

#[test]
fn rerun_returns_the_scheduled_stages_with_camel_case_fields() {
    let app = pipeline_app();

    let stages = invoke(
        &main_window(&app),
        "plugin:pipeline|rerun",
        json!({ "meetingId": "meeting-1", "stage": null }),
    );

    let pending = |stage: &str| {
        json!({
            "stage": stage,
            "status": "pending",
            "attempts": 0,
            "retryAt": null,
            "message": null,
            "updatedAt": NOW,
        })
    };
    assert_eq!(
        stages,
        Ok(json!([
            pending("finalize_audio"),
            pending("screen_video"),
            pending("final_transcription"),
            pending("diarize"),
            pending("identify_speakers"),
            pending("vocab_fixups"),
            pending("enhance_notes"),
            pending("extract_action_items"),
            pending("auto_title"),
            pending("auto_tag"),
            pending("auto_folder"),
            pending("index"),
            pending("insights"),
            pending("update_entities"),
            pending("automations"),
            pending("auto_export")
        ]))
    );
}

#[test]
fn rerun_emits_pipeline_progress_with_the_meeting_stage_and_status() {
    let app = pipeline_app();
    let (sender, received) = mpsc::channel();
    app.listen_any("plugin:pipeline:pipeline-progress", move |event| {
        sender
            .send(event.payload().to_owned())
            .expect("the test is still listening");
    });

    let args = json!({ "meetingId": "meeting-1", "stage": "enhance_notes" });
    invoke(&main_window(&app), "plugin:pipeline|rerun", args).unwrap();

    let payload: Value =
        serde_json::from_str(&received.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
    assert_eq!(
        payload,
        json!({ "meetingId": "meeting-1", "stage": "enhance_notes", "status": "pending" })
    );
}

#[test]
fn errors_arrive_as_app_errors_with_a_code() {
    let app = pipeline_app();
    let window = main_window(&app);

    let missing = invoke(
        &window,
        "plugin:pipeline|rerun",
        json!({ "meetingId": "meeting-x", "stage": null }),
    );
    let unknown = invoke(
        &window,
        "plugin:pipeline|rerun",
        json!({ "meetingId": "meeting-1", "stage": "translate" }),
    );

    assert_eq!(
        missing,
        Err(json!({ "code": "not_found", "message": "meeting meeting-x was not found" }))
    );
    assert_eq!(
        unknown,
        Err(json!({ "code": "invalid", "message": "the pipeline has no stage named translate" }))
    );
}
