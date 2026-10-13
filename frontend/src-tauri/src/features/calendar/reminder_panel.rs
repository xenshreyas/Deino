//! macOS: the reminder as a non-activating `NSPanel`. It floats at status
//! level, joins every Space and full-screen app, and becomes key on a click
//! without activating Deino, so the user's current app keeps focus (§3C).

use tauri::{AppHandle, LogicalPosition, LogicalSize, Position, Size, WebviewUrl, Wry};
use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask};

use super::{HEIGHT, REMINDER_LABEL, REMINDER_PAGE, WIDTH, place};

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
        panel!(ReminderPanel {
            config: {
                can_become_key_window: true,
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
    let panel = match app.get_webview_panel(REMINDER_LABEL) {
        Ok(panel) => {
            place(app, position);
            panel
        }
        Err(_) => build(app, position)?,
    };
    panel.show();
    Ok(())
}

/// Hides the panel; nothing to do before the first reminder.
pub(super) fn hide(app: &AppHandle<Wry>) {
    if let Ok(panel) = app.get_webview_panel(REMINDER_LABEL) {
        panel.hide();
    }
}

fn build(
    app: &AppHandle<Wry>,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<tauri_nspanel::PanelHandle<Wry>> {
    let mut builder = PanelBuilder::<_, class::ReminderPanel<Wry>>::new(app, REMINDER_LABEL)
        .url(WebviewUrl::App(REMINDER_PAGE.into()))
        .title("Meeting starting")
        .size(Size::Logical(LogicalSize::new(WIDTH, HEIGHT)))
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
                .full_screen_auxiliary(),
        )
        .with_window(|window| {
            window
                .decorations(false)
                .transparent(true)
                .resizable(false)
                .skip_taskbar(true)
                .shadow(false)
                .visible(false)
                .focused(false)
        });
    if let Some(position) = position {
        builder = builder.position(Position::Logical(position));
    }
    builder.build()
}
