//! Contract tests: the JSON the ElevenLabs card sends and receives, through
//! the real plugin, ACL and error mapping, over an in-memory Keychain and a
//! fake network that answers with recorded ElevenLabs fixtures (nothing is
//! sent anywhere).

use std::sync::Arc;

use async_trait::async_trait;
use deino_cloud_transcription::{CloudSettings, ElevenLabs, ElevenLabsParts};
use deino_core::final_transcription::{
    FakeOriginalTranscripts, OriginalTranscriptDeps, OriginalTranscriptService,
};
use deino_core::segments::FakeSegmentRepo;
use deino_core::settings::{CloudTranscriptionModel, CloudTranscriptionProvider};
use deino_domain::ServiceId;
use deino_net::{FakeNetClient, FakeReply, FakeServicePolicy, ServicePolicy, Status};
use deino_secrets::InMemorySecretStore;
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::Manager;
use tauri::WebviewWindow;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// The cloud option, switched on.
struct On;

#[async_trait]
impl CloudSettings for On {
    async fn cloud(&self) -> (CloudTranscriptionProvider, CloudTranscriptionModel) {
        (
            CloudTranscriptionProvider::ElevenLabs,
            CloudTranscriptionModel::ScribeV2,
        )
    }
}

/// A recorded answer from `crates/deino-cloud-transcription/fixtures/`.
fn recorded(status: u16, file: &str) -> FakeReply {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/deino-cloud-transcription/fixtures/elevenlabs")
        .join(file);
    let body = std::fs::read(path).unwrap();
    FakeReply::ok(body).with_status(Status::new(status))
}

struct World {
    window: WebviewWindow<MockRuntime>,
    client: Arc<FakeNetClient>,
    policy: Arc<FakeServicePolicy>,
}

fn world() -> World {
    let app = mock_app(mock_builder().plugin(super::init()));
    let client = Arc::new(FakeNetClient::default());
    let policy = Arc::new(FakeServicePolicy::default());
    app.manage(Arc::new(ElevenLabs::new(ElevenLabsParts {
        client: client.clone(),
        secrets: Arc::new(InMemorySecretStore::default()),
        switch: policy.clone(),
        settings: Arc::new(On),
    })));
    let segments = Arc::new(FakeSegmentRepo::default());
    app.manage(Arc::new(OriginalTranscriptService::new(
        OriginalTranscriptDeps {
            originals: Arc::new(FakeOriginalTranscripts::default()),
            segments: segments.clone(),
            writer: segments.clone(),
            speakers: segments,
            clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
        },
    )));
    World {
        window: main_window(&app),
        client,
        policy,
    }
}

impl World {
    fn call(&self, command: &str, args: Value) -> Result<Value, Value> {
        invoke(
            &self.window,
            &format!("plugin:cloud-transcription|{command}"),
            args,
        )
    }

    fn allowed(&self) -> bool {
        tauri::async_runtime::block_on(self.policy.grant(ServiceId::ElevenLabs)).enabled
    }
}

#[test]
fn a_key_is_saved_checked_and_forgotten_without_ever_coming_back() {
    let world = world();
    world.client.reply(recorded(200, "user.json"));
    world
        .client
        .reply(recorded(401, "error-401-invalid-key.json"));

    let before = world.call("key_status", json!({})).unwrap();
    world
        .call("save_key", json!({ "key": "xi-secret" }))
        .unwrap();
    let saved = world.call("key_status", json!({})).unwrap();
    let switched_on = world.allowed();
    let checked = world.call("check_key", json!({})).unwrap();
    let refused = world.call("check_key", json!({})).unwrap_err();
    world.call("forget_key", json!({})).unwrap();

    assert_eq!((before, saved), (json!("missing"), json!("stored")));
    assert!(switched_on);
    assert_eq!(checked, Value::Null);
    assert_eq!(refused["code"], "permission_denied");
    assert!(!refused.to_string().contains("xi-secret"));
    assert!(!world.allowed());
    assert_eq!(world.call("key_status", json!({})).unwrap(), "missing");
}

#[test]
fn a_blank_key_is_invalid_and_checking_without_one_is_unavailable() {
    let world = world();

    let blank = world.call("save_key", json!({ "key": "  " })).unwrap_err();
    let unsaved = world.call("check_key", json!({})).unwrap_err();

    assert_eq!(blank["code"], "invalid");
    assert_eq!(unsaved["code"], "unavailable");
    assert!(world.client.requests().is_empty());
}

#[test]
fn a_meeting_without_a_kept_original_says_so() {
    let world = world();

    let kept = world.call("has_original", json!({ "meetingId": "meeting-a" }));

    assert_eq!(kept, Ok(json!(false)));
}
