//! Other platforms: the reminder as an always-on-top window that is not
//! focused when shown. Deino ships on macOS; this keeps the app compiling
//! and behaving sensibly elsewhere.

use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use super::{HEIGHT, REMINDER_LABEL, REMINDER_PAGE, WIDTH, place};

/// Shows the window at `position`, building it the first time.
pub(super) fn show(
    app: &AppHandle<Wry>,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<()> {
    let window = match app.get_webview_window(REMINDER_LABEL) {
        Some(window) => {
            place(app, position);
            window
        }
        None => build(app, position)?,
    };
    window.show()
}

/// Hides the window; nothing to do before the first reminder.
pub(super) fn hide(app: &AppHandle<Wry>) {
    let Some(window) = app.get_webview_window(REMINDER_LABEL) else {
        return;
    };
    if let Err(error) = window.hide() {
        log::debug!(target: "calendar", "could not hide the reminder: {error}");
    }
}

fn build(
    app: &AppHandle<Wry>,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<tauri::WebviewWindow<Wry>> {
    let mut builder =
        WebviewWindowBuilder::new(app, REMINDER_LABEL, WebviewUrl::App(REMINDER_PAGE.into()))
            .title("Meeting starting")
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
