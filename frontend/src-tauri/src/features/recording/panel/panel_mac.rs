//! macOS: the record panel as a non-activating `NSPanel` that can never be
//! the key or main window. It floats at status level, joins every Space and
//! full-screen app, and passes clicks through until the pointer is over
//! the pill. Built once, then only shown and hidden.

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Position, Size, WebviewUrl, Wry};
use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask};

use super::{FRAME, PANEL_LABEL, PANEL_PAGE};

#[allow(
    unsafe_code,
    clippy::pedantic,
    missing_docs,
    reason = "tauri_panel! expands to the Objective-C declaration of the panel class"
)]
mod class {
    // The macro's expansion calls `Manager::app_handle`.
    use tauri::Manager;

    tauri_nspanel::tauri_panel! {
        panel!(RecordPanel {
            config: {
                can_become_key_window: false,
                can_become_main_window: false,
                is_floating_panel: true
            }
        })
    }
}

/// Shows the panel at `position`, building it the first time.
pub(super) fn show(
    app: &AppHandle<Wry>,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<()> {
    let panel = match app.get_webview_panel(PANEL_LABEL) {
        Ok(panel) => {
            if let (Some(window), Some(position)) = (app.get_webview_window(PANEL_LABEL), position)
            {
                window.set_position(position)?;
            }
            panel
        }
        Err(_) => build(app, position)?,
    };
    panel.set_ignores_mouse_events(true);
    panel.show();
    Ok(())
}

/// Hides the panel; nothing to do before the first recording.
pub(super) fn hide(app: &AppHandle<Wry>) {
    if let Ok(panel) = app.get_webview_panel(PANEL_LABEL) {
        panel.hide();
    }
}

fn build(
    app: &AppHandle<Wry>,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<tauri_nspanel::PanelHandle<Wry>> {
    let mut builder = PanelBuilder::<_, class::RecordPanel<Wry>>::new(app, PANEL_LABEL)
        .url(WebviewUrl::App(PANEL_PAGE.into()))
        .title("Recording")
        .size(Size::Logical(LogicalSize::new(FRAME.width, FRAME.height)))
        .level(PanelLevel::Status)
        .has_shadow(false)
        .transparent(true)
        .hides_on_deactivate(false)
        // Never `no_activate(true)`: tauri-nspanel implements it by switching the
        // app to background-only while building, which hides every Deino window
        // (this panel too) until the user reopens the app. The style keeps focus.
        .style_mask(StyleMask::empty().borderless().nonactivating_panel())
        .collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .stationary(),
        )
        .with_window(|window| {
            window
                .decorations(false)
                .transparent(true)
                .resizable(false)
                .skip_taskbar(true)
                .shadow(false)
                .accept_first_mouse(true)
                .visible(false)
                .focused(false)
        });
    if let Some(position) = position {
        builder = builder.position(Position::Logical(position));
    }
    builder.build()
}
