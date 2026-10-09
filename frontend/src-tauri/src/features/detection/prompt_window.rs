//! Other platforms: the prompt as an always-on-top window that is not
//! focused when shown. Detection itself is macOS-only, so this only keeps
//! the app compiling and behaving sensibly elsewhere.

use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use super::{HEIGHT, PROMPT_LABEL, PROMPT_PAGE, WIDTH, place};

/// Shows the window at `position`, building it the first time.
pub(super) fn show(
    app: &AppHandle<Wry>,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<()> {
    let window = match app.get_webview_window(PROMPT_LABEL) {
        Some(window) => {
            place(app, position);
            window
        }
        None => build(app, position)?,
    };
    window.show()
}

/// Hides the window; nothing to do before the first prompt.
pub(super) fn hide(app: &AppHandle<Wry>) {
    let Some(window) = app.get_webview_window(PROMPT_LABEL) else {
        return;
    };
    if let Err(error) = window.hide() {
        log::debug!(target: "detect", "could not hide the meeting prompt: {error}");
    }
}

fn build(
    app: &AppHandle<Wry>,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<tauri::WebviewWindow<Wry>> {
    let mut builder =
        WebviewWindowBuilder::new(app, PROMPT_LABEL, WebviewUrl::App(PROMPT_PAGE.into()))
            .title("Meeting detected")
            .inner_size(WIDTH, HEIGHT)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .focused(false)
            .visible(false);
    if let Some(position) = position {
        builder = builder.position(position.x, position.y);
    }
    builder.build()
}
