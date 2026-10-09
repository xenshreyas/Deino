//! Contract tests: the `transcription` commands' JSON in and out, and the
//! model events, through the real plugin and ACL on Tauri's mock runtime.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use deino_core::models::{
    Catalog, FakeByteSource, FakeServer, ModelService, ModelServiceParts, ModelStore,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Listener, Manager};

use super::TauriModelEvents;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

const ID: &str = "tiny-Q8_0";

fn content() -> Vec<u8> {
    (0..10_000_u32).map(|i| (i % 239) as u8).collect()
}

/// A one-file catalog whose file is `content()`.
fn catalog() -> Catalog {
    let bytes = content();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .fold(String::new(), |hex, byte| hex + &format!("{byte:02x}"));
    let json = format!(
        r#"{{"models":[{{"repo":"test/tiny","revision":"abc","name":"Tiny","license":"MIT",
        "default_quant":"Q8_0","files":[{{"filename":"{ID}.gguf","quant":"Q8_0",
        "size_bytes":{},"sha256":"{sha}"}}]}}]}}"#,
        bytes.len()
    );
    Catalog::parse(&json).unwrap()
}

fn transcription_app(dir: &tempfile::TempDir) -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let service = ModelService::new(ModelServiceParts {
        catalog: catalog(),
        store: ModelStore::new(dir.path()),
        source: Arc::new(FakeByteSource::new(content(), FakeServer::Ranged)),
        events: Arc::new(TauriModelEvents::new(app.handle().clone())),
        memory_bytes: 64 << 30,
    });
    app.manage(Arc::new(service));
    app
}

#[test]
fn list_models_returns_each_file_with_its_state_and_recommendation() {
    let dir = tempfile::tempdir().unwrap();
    let app = transcription_app(&dir);

    let listed = invoke(
        &main_window(&app),
        "plugin:transcription|list_models",
        Value::Null,
    );

    assert_eq!(
        listed,
        Ok(json!([{
            "id": ID,
            "name": "Tiny",
            "quant": "Q8_0",
            "sizeBytes": 10_000,
            "state": { "kind": "missing" },
            "recommended": true,
        }]))
    );
}

#[test]
fn download_model_makes_the_file_ready_and_announces_the_change() {
    let dir = tempfile::tempdir().unwrap();
    let app = transcription_app(&dir);
    let (sender, changed) = mpsc::channel();
    app.listen_any("plugin:transcription:model-changed", move |event| {
        sender.send(event.payload().to_owned()).unwrap();
    });
    let window = main_window(&app);

    let downloaded = invoke(
        &window,
        "plugin:transcription|download_model",
        json!({ "id": ID }),
    );
    let listed = invoke(&window, "plugin:transcription|list_models", Value::Null).unwrap();

    assert_eq!(downloaded, Ok(Value::Null));
    assert_eq!(listed[0]["state"], json!({ "kind": "ready" }));
    let payload: Value =
        serde_json::from_str(&changed.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
    assert_eq!(payload, json!({ "id": ID }));
}

#[test]
fn unknown_models_are_not_found_and_cancel_without_a_download_is_false() {
    let dir = tempfile::tempdir().unwrap();
    let app = transcription_app(&dir);
    let window = main_window(&app);

    let unknown = invoke(
        &window,
        "plugin:transcription|delete_model",
        json!({ "id": "nope" }),
    );
    let cancelled = invoke(
        &window,
        "plugin:transcription|cancel_download",
        json!({ "id": ID }),
    );

    assert_eq!(
        unknown,
        Err(json!({ "code": "not_found", "message": "unknown model nope" }))
    );
    assert_eq!(cancelled, Ok(Value::Bool(false)));
}
