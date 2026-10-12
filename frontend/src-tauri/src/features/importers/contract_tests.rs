//! Contract tests: the JSON Settings → Import sends and receives, through
//! the real plugin, ACL and error mapping, over SQLite, an in-memory
//! Keychain and a fake network (nothing is sent anywhere).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use deino_core::importers::{ImportJobs, ImportWorld, NoImportEvents};
use deino_domain::{Clock, ServiceId};
use deino_importers::ImportSources;
use deino_net::{EgressLog, FakeNetClient, FakeServicePolicy, Net, NetParts, ServicePolicy};
use deino_secrets::InMemorySecretStore;
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{Manager, WebviewWindow};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// The folder fixtures `deino-importers` tests with.
fn fixtures() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/deino-importers/fixtures/folder")
        .to_string_lossy()
        .into_owned()
}

fn window() -> (WebviewWindow<MockRuntime>, Arc<FakeServicePolicy>) {
    let app = mock_app(mock_builder().plugin(super::init()));
    let world = tauri::async_runtime::block_on(ImportWorld::new());
    let policy = Arc::new(FakeServicePolicy::default());
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
    let net = Net::new(NetParts {
        client: Arc::new(FakeNetClient::default()),
        policy: policy.clone(),
        egress: Arc::new(EgressLog::default()),
        clock,
    });
    let sources = ImportSources::new(net, Arc::new(InMemorySecretStore::default()), policy.clone());
    app.manage(Arc::new(sources));
    app.manage(Arc::new(ImportJobs::new(world.runner, Arc::new(NoImportEvents))));
    (main_window(&app), policy)
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:importers|{command}"), args)
}

#[test]
fn a_saved_key_marks_its_source_ready_and_switches_its_service_on() {
    let (window, policy) = window();

    call(&window, "save_key", json!({ "source": "granola", "key": "grn_abc" })).unwrap();
    let sources = call(&window, "sources", json!({})).unwrap();
    let enabled = tauri::async_runtime::block_on(policy.grant(ServiceId::Granola)).enabled;
    call(&window, "forget_key", json!({ "source": "granola" })).unwrap();
    let after = call(&window, "sources", json!({})).unwrap();

    assert_eq!(sources.as_array().unwrap().len(), 5);
    assert_eq!(sources[0], json!({ "source": "granola", "key": "stored" }));
    assert_eq!(sources[4], json!({ "source": "folder", "key": "stored" }));
    assert!(enabled);
    assert_eq!(after[0]["key"], "missing");
    assert!(sources.to_string().find("grn_abc").is_none());
}

#[test]
fn a_provider_without_a_key_cannot_be_previewed() {
    let (window, _) = window();

    let error = call(&window, "preview", json!({ "target": { "source": "fathom", "folder": null } }))
        .unwrap_err();

    assert_eq!(error["code"], "not_found");
}

#[test]
fn a_folder_is_previewed_then_imported_in_the_background() {
    let (window, _) = window();
    let target = json!({ "target": { "source": "folder", "folder": fixtures() } });

    let preview = call(&window, "preview", target.clone()).unwrap();
    let idle = call(&window, "status", json!({})).unwrap();
    let first = call(&window, "start", target).unwrap();
    let done = tauri::async_runtime::block_on(async {
        for _ in 0..200 {
            let status = call(&window, "status", json!({})).unwrap();
            if status["phase"] == "finished" {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Value::Null
    });

    assert_eq!(preview, json!({ "source": "folder", "total": 3, "alreadyImported": 0, "new": 3 }));
    assert_eq!(idle["phase"], "idle");
    assert_eq!(first["phase"], "listing");
    assert_eq!((done["imported"].clone(), done["failed"].clone()), (json!(2), json!(1)));
    assert_eq!(call(&window, "cancel", json!({})).unwrap(), json!(false));
}
