//! Contract tests: the JSON Settings → Developer sends and receives.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::cli_setup::{CliSetupService, FsLinks};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::test::{MockRuntime, mock_builder};
use tempfile::TempDir;

use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// A home folder and a bundled CLI, both temporary.
struct World {
    home: TempDir,
    bundled: PathBuf,
}

fn world() -> World {
    let home = tempfile::tempdir().unwrap();
    let bundled = home.path().join("Deino.app-cli");
    std::fs::write(&bundled, "#!/bin/sh\n").unwrap();
    World { home, bundled }
}

fn window(world: &World) -> WebviewWindow<MockRuntime> {
    let service = CliSetupService::new(
        Arc::new(FsLinks),
        Some(world.bundled.clone()),
        world.home.path().to_path_buf(),
    );
    main_window(&mock_app(
        mock_builder()
            .plugin(super::init())
            .manage(Arc::new(service)),
    ))
}

#[test]
fn status_reports_the_bundled_cli_targets_and_snippets() {
    let world = world();

    let status = invoke(&window(&world), "plugin:developer|cli_status", json!({})).unwrap();

    let bundled = world.bundled.display().to_string();
    assert_eq!(status["bundled"], Value::String(bundled.clone()));
    assert_eq!(status["targets"][0]["dir"], "user_bin");
    assert_eq!(status["targets"][0]["state"], json!({ "kind": "missing" }));
    assert_eq!(status["targets"][1]["path"], "/usr/local/bin/deino");
    assert_eq!(
        status["claudeCode"],
        Value::String(format!("claude mcp add deino -- {bundled} mcp"))
    );
    assert!(
        status["claudeDesktop"]
            .as_str()
            .unwrap()
            .contains("\"mcpServers\"")
    );
}

#[test]
fn install_links_the_cli_into_the_users_bin_folder() {
    let world = world();
    let link = world.home.path().join(".local/bin/deino");

    let status = invoke(
        &window(&world),
        "plugin:developer|install_cli",
        json!({ "dir": "user_bin" }),
    )
    .unwrap();

    assert_eq!(std::fs::read_link(&link).unwrap(), world.bundled);
    assert_eq!(
        status["targets"][0]["state"],
        json!({ "kind": "installed" })
    );
    assert_eq!(status["command"], Value::String(link.display().to_string()));
}

#[test]
fn a_file_in_the_way_is_a_conflict() {
    let world = world();
    let link = world.home.path().join(".local/bin/deino");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::fs::write(&link, "mine").unwrap();

    let error = invoke(
        &window(&world),
        "plugin:developer|install_cli",
        json!({ "dir": "user_bin" }),
    )
    .unwrap_err();

    assert_eq!(error["code"], "conflict");
    assert_eq!(std::fs::read_to_string(&link).unwrap(), "mine");
}
