//! The prompt window: a small, always-on-top card in the top-right corner of
//! the main display (ui.md, Granola §2.3) that never takes focus from the
//! call. On macOS it is a non-activating `NSPanel` at status level on every
//! Space (Handy's overlay pattern, through `tauri-nspanel`); elsewhere a
//! plain always-on-top window that is not focused when shown.
//!
//! The window is created on the first prompt and then only hidden and shown,
//! so later prompts appear instantly. Its page (`detection-prompt.html`)
//! renders from `plugin:detection` events and `current`.

use tauri::{AppHandle, LogicalPosition, Manager, Wry};

/// The window's label; `capabilities/detection-prompt.json` grants it.
pub const PROMPT_LABEL: &str = "detection_prompt";

/// The page the window loads (a Vite entry next to `index.html`).
const PROMPT_PAGE: &str = "detection-prompt.html";

/// The window, in logical pixels. The card inside leaves room for its shadow.
const WIDTH: f64 = 376.0;
const HEIGHT: f64 = 184.0;

/// The gap below the menu bar, minus the shadow room. There is none on the
/// right: the window reaches the screen's edge so the card can slide in from
/// off-screen (its page keeps the card inset from that edge).
const INSET: f64 = 8.0;

/// Shows the prompt window, creating it the first time. Runs on the main
/// thread, as `AppKit` requires.
pub(crate) fn show(app: &AppHandle<Wry>) {
    let handle = app.clone();
    let shown = app.run_on_main_thread(move || {
        let position = top_right(&handle);
        if let Err(error) = platform::show(&handle, position) {
            log::error!(target: "detect", "could not show the meeting prompt: {error}");
        }
    });
    if let Err(error) = shown {
        log::error!(target: "detect", "could not reach the main thread: {error}");
    }
}

/// Hides the prompt window, if it exists.
pub(crate) fn hide(app: &AppHandle<Wry>) {
    let handle = app.clone();
    let hidden = app.run_on_main_thread(move || platform::hide(&handle));
    if let Err(error) = hidden {
        log::error!(target: "detect", "could not reach the main thread: {error}");
    }
}

/// The top-right corner of the primary display's work area (below the menu bar).
fn top_right(app: &AppHandle<Wry>) -> Option<LogicalPosition<f64>> {
    let monitor = app.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let right = f64::from(area.position.x) / scale + f64::from(area.size.width) / scale;
    let top = f64::from(area.position.y) / scale;
    Some(LogicalPosition::new(right - WIDTH, top + INSET))
}

/// Moves the existing window to `position`.
fn place(app: &AppHandle<Wry>, position: Option<LogicalPosition<f64>>) {
    let (Some(window), Some(position)) = (app.get_webview_window(PROMPT_LABEL), position) else {
        return;
    };
    if let Err(error) = window.set_position(position) {
        log::debug!(target: "detect", "could not move the meeting prompt: {error}");
    }
}

#[cfg(target_os = "macos")]
#[path = "prompt_panel.rs"]
mod platform;

#[cfg(not(target_os = "macos"))]
#[path = "prompt_window.rs"]
mod platform;
