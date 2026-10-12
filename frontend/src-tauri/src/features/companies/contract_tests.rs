//! Contract tests: the JSON of the `companies` commands through the real
//! plugin, ACL and error mapping, over SQLite, with the network fake.

use std::sync::Arc;

use deino_core::entities::{
    CompanyService, EntityPorts, EntitySummarizer, FaviconSource, FsLogoFiles, LogoPorts,
    LogoService,
};
use deino_domain::{Clock, ServiceId};
use deino_net::{EgressLog, FakeNetClient, FakeServicePolicy, Net, NetParts};
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewWindow};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-11T12:00:00Z";

/// An app with the `companies` plugin over an empty database, no LLM, and
/// company logos switched off (the default).
fn companies_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(test_db());
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at(NOW)));
    let ports = EntityPorts::sqlite(&database, Arc::clone(&clock));
    let net = Net::new(NetParts {
        client: Arc::new(FakeNetClient::default()),
        policy: Arc::new(FakeServicePolicy::default()),
        egress: Arc::new(EgressLog::default()),
        clock,
    });
    let logos = LogoService::new(LogoPorts {
        companies: Arc::clone(&ports.companies),
        directory: Arc::clone(&ports.directory),
        writes: Arc::clone(&ports.writes),
        source: Arc::new(FaviconSource::new(Arc::new(
            net.scoped(ServiceId::Favicons),
        ))),
        files: Arc::new(FsLogoFiles::new(
            std::env::temp_dir().join("deino-logo-contract"),
        )),
        clock: Arc::clone(&ports.clock),
    });
    app.manage(Arc::new(CompanyService::new(ports.clone())));
    app.manage(Arc::new(EntitySummarizer::new(ports, None)));
    app.manage(Arc::new(logos));
    app
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:companies|{command}"), args)
}

fn create(window: &WebviewWindow<MockRuntime>, name: &str, domain: &str) -> Value {
    let fields = json!({ "name": name, "domain": domain });
    call(window, "create", json!({ "fields": fields })).unwrap()
}

#[test]
fn companies_are_created_listed_and_paged_in_camel_case() {
    let app = companies_app();
    let window = main_window(&app);

    let acme = create(&window, "Acme", "https://www.acme.com");
    let list = call(&window, "list", json!({})).unwrap();
    let page = call(&window, "page", json!({ "id": acme["id"] })).unwrap();

    assert_eq!(acme["domain"], "acme.com");
    assert_eq!(acme["logoCheckedAt"], json!(null));
    assert_eq!(list[0]["peopleCount"], 0);
    assert_eq!(list[0]["lastMetAt"], json!(null));
    assert_eq!(page["company"]["name"], "Acme");
    assert_eq!(page["people"], json!([]));
}

#[test]
fn edits_merges_and_refusals_map_to_error_codes() {
    let app = companies_app();
    let window = main_window(&app);
    let acme = create(&window, "Acme", "acme.com");
    let europe = create(&window, "Acme Europe", "acme.eu");

    let clash = call(
        &window,
        "update",
        json!({ "id": europe["id"], "fields": { "name": "X", "domain": "acme.com" } }),
    )
    .unwrap_err();
    let merged = call(
        &window,
        "merge",
        json!({ "survivor": acme["id"], "absorbed": europe["id"] }),
    )
    .unwrap();
    call(
        &window,
        "set_notes",
        json!({ "id": acme["id"], "notes": "Q1 renewal" }),
    )
    .unwrap();
    let summary = call(&window, "refresh_summary", json!({ "id": acme["id"] })).unwrap();
    let logo = call(
        &window,
        "fetch_logo",
        json!({ "id": acme["id"], "refresh": false }),
    )
    .unwrap_err();

    assert_eq!(clash["code"], "conflict");
    assert_eq!(merged["id"], acme["id"]);
    assert_eq!(summary, "no_model");
    assert_eq!(
        logo["code"], "permission_denied",
        "company logos are off by default"
    );
}
