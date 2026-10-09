//! IPC tests on Tauri's mock runtime: the harness every feature's contract
//! tests use, and the ACL tests.
//!
//! The app is built from the real `tauri.conf.json` and the ACL that
//! `build.rs` generated, so a passing test proves that the manifest entry, the
//! capability grant and the command name line up, and pins the JSON shape the
//! frontend relies on.

use serde_json::Value;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;
use tauri::{App, Builder, Manager, WebviewUrl, WebviewWindow};

use super::system;

/// Builds an app from `builder` with the real config and generated ACL.
pub(crate) fn mock_app(builder: Builder<MockRuntime>) -> App<MockRuntime> {
    // `test = true` skips the macOS Info.plist embed that `lib.rs` already emits.
    builder
        .build(tauri::generate_context!(test = true))
        .expect("mock app builds from the real config")
}

/// The `main` window, which `capabilities/` grants every feature plugin.
pub(crate) fn main_window(app: &App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
            .build()
            .expect("main window")
    })
}

/// Invokes `command` with JSON `args` the way the UI does, returning the
/// command's JSON result or its JSON error.
pub(crate) fn invoke(
    window: &WebviewWindow<MockRuntime>,
    command: &str,
    args: Value,
) -> Result<Value, Value> {
    let url = if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    let request = InvokeRequest {
        cmd: command.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: url.parse().expect("valid local URL"),
        body: InvokeBody::Json(args),
        headers: tauri::http::HeaderMap::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    get_ipc_response(window, request).map(|body| body.deserialize().expect("a JSON response"))
}

#[test]
fn granted_plugin_command_is_callable() {
    let app = mock_app(mock_builder().plugin(system::init()));

    let version = invoke(&main_window(&app), "plugin:system|app_version", Value::Null);

    // Also guards that Cargo.toml and tauri.conf.json carry the same version.
    assert_eq!(
        version,
        Ok(Value::from(app.package_info().version.to_string()))
    );
}

#[test]
fn plugin_command_is_denied_to_a_window_without_the_capability() {
    let app = mock_app(mock_builder().plugin(system::init()));
    let window = tauri::WebviewWindowBuilder::new(&app, "unlisted", WebviewUrl::default())
        .build()
        .expect("unlisted window");

    let error = invoke(&window, "plugin:system|app_version", Value::Null).unwrap_err();

    assert!(
        error.to_string().contains("not allowed"),
        "unexpected error: {error}"
    );
}
