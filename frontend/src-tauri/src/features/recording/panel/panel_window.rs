//! Other platforms: the record panel as an always-on-top window that is not
//! focused when shown. Recording is macOS-first; this keeps the app
//! compiling and behaving sensibly elsewhere.

use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use super::{FRAME, PANEL_LABEL, PANEL_PAGE};

/// Shows the window at `position`, building it the first time.
pub(super) fn show(
    app: &AppHandle<Wry>,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<()> {
    let window = match app.get_webview_window(PANEL_LABEL) {
        Some(window) => window,
        None => build(app)?,
    };
    if let Some(position) = position {
        window.set_position(position)?;
    }
    window.set_ignore_cursor_events(true)?;
    window.show()
}

/// Hides the window; nothing to do before the first recording.
pub(super) fn hide(app: &AppHandle<Wry>) {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };
    if let Err(error) = window.hide() {
        log::debug!(target: "recording", "could not hide the record panel: {error}");
    }
}

fn build(app: &AppHandle<Wry>) -> tauri::Result<tauri::WebviewWindow<Wry>> {
    WebviewWindowBuilder::new(app, PANEL_LABEL, WebviewUrl::App(PANEL_PAGE.into()))
        .title("Recording")
        .inner_size(FRAME.width, FRAME.height)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .skip_taskbar(true)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()
}
