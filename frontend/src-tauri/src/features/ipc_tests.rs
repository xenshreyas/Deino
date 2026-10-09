//! End-to-end IPC tests for feature plugins on Tauri's mock runtime.
//!
//! The app is built from the real `tauri.conf.json` and the ACL that
//! `build.rs` generated, so these tests prove that a manifest entry plus a
//! capability grant make `plugin:<feature>|<command>` callable, and that the
//! ACL denies it to a window the capability does not cover.

use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, Manager, WebviewWindow};

use super::system;

fn mock_app() -> App<tauri::test::MockRuntime> {
    mock_builder()
        .plugin(system::init())
        // `test = true` skips the macOS Info.plist embed that `lib.rs` already emits.
        .build(tauri::generate_context!(test = true))
        .expect("mock app builds from the real config")
}

fn main_window(app: &App<tauri::test::MockRuntime>) -> WebviewWindow<tauri::test::MockRuntime> {
    app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(app, "main", Default::default())
            .build()
            .expect("main window")
    })
}

fn invoke_request(cmd: &str) -> InvokeRequest {
    let url = if cfg!(windows) { "http://tauri.localhost" } else { "tauri://localhost" };
    InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: url.parse().expect("valid local URL"),
        body: InvokeBody::default(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

#[test]
fn granted_plugin_command_is_callable() {
    let app = mock_app();
    let window = main_window(&app);

    let response = get_ipc_response(&window, invoke_request("plugin:system|app_version"))
        .expect("plugin:system|app_version is granted by system:default");

    let version: String = response.deserialize().expect("version string");
    assert_eq!(version, app.package_info().version.to_string());
}

#[test]
fn plugin_command_is_denied_to_a_window_without_the_capability() {
    let app = mock_app();
    let window = tauri::WebviewWindowBuilder::new(&app, "unlisted", Default::default())
        .build()
        .expect("unlisted window");

    let response = get_ipc_response(&window, invoke_request("plugin:system|app_version"));

    let error = response.expect_err("the ACL must deny windows outside capabilities/default.json");
    assert!(error.to_string().contains("not allowed"), "unexpected error: {error}");
}
