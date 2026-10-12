//! The record panel (CAP-10, ui.md "Record pill"): Granola's nub. A small
//! transparent window at the edge of the screen, shown for as long as a
//! recording runs, floating above every app and Space and never taking
//! focus from the call.
//!
//! - **Never steals focus.** On macOS it is a non-activating `NSPanel`
//!   (`tauri-nspanel`, as the meeting prompt) at status level that cannot
//!   become the key or main window, so clicking it leaves the call app
//!   active and keeps the keyboard where it was; WebKit takes the first
//!   click (`accept_first_mouse`) so its buttons work all the same.
//! - **Clicks pass through around the pill.** The window is a fixed size
//!   (resizing a transparent window flashes a stale frame), larger than the
//!   pill; it ignores the mouse except over the pill's rectangle, which the
//!   page reports ([`set_region`]). A driver thread ([`driver`]) watches the
//!   pointer to switch that, tells the page about hover, sends the level
//!   meter about 30 times a second and snaps the panel to the nearer edge
//!   when a drag ends ([`placement`]), remembering where in the settings.
//! - The page is `record-panel.html`, with its own capability that grants
//!   the `recording` plugin.

mod driver;
pub(crate) mod placement;

use std::sync::{Arc, Mutex, PoisonError};

use deino_core::settings::{PanelEdge, PanelPosition};
use deino_recording::{RecordingPhase, RecordingSnapshot};
use tauri::{AppHandle, LogicalPosition, Manager, Wry};

use self::driver::Driver;
use self::placement::{DEFAULT_POSITION, Frame, Rect, origin};
use super::adapter::Follower;

/// The window's label; `capabilities/record-panel.json` grants it.
pub const PANEL_LABEL: &str = "record_panel";

/// The page the window loads (a Vite entry next to `index.html`).
const PANEL_PAGE: &str = "record-panel.html";

/// The window's size, and the transparent margin around the pill for its
/// shadow. The pill morphs inside; the window never resizes. Tall enough
/// for the back-to-back offer under the pill; the window lets clicks
/// through everywhere but the pill and the card.
pub const FRAME: Frame = Frame {
    width: 380.0,
    height: 176.0,
    margin: 12.0,
};

/// What the panel's driver shares with the commands.
#[derive(Default)]
pub struct PanelHost {
    /// The pill's rectangle inside the window, in logical points.
    region: Mutex<Option<Rect>>,
    /// The screen edge the panel hugs.
    edge: Mutex<PanelEdge>,
    driver: Mutex<Option<Driver>>,
}

impl PanelHost {
    /// The pill's rectangle, once the page has reported it.
    fn region(&self) -> Option<Rect> {
        *self.region.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Remembers the edge the panel was placed against.
    fn set_edge(&self, edge: PanelEdge) {
        *self.edge.lock().unwrap_or_else(PoisonError::into_inner) = edge;
    }
}

/// Records the pill's rectangle (the page reports it whenever it changes);
/// returns the edge the panel hugs, so the page lays the pill out against it.
pub(crate) fn set_region(host: &PanelHost, region: Rect) -> PanelEdge {
    *host.region.lock().unwrap_or_else(PoisonError::into_inner) = Some(region);
    *host.edge.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Shows the panel while a recording runs and hides it after.
pub struct RecordPanel;

impl Follower<Wry> for RecordPanel {
    fn follow(
        &self,
        app: &AppHandle<Wry>,
        previous: &RecordingSnapshot,
        state: &RecordingSnapshot,
    ) {
        let was_on = previous.phase != RecordingPhase::Idle;
        let is_on = state.phase != RecordingPhase::Idle;
        match (was_on, is_on) {
            (false, true) => show(app),
            (true, false) => hide(app),
            _ => {}
        }
    }
}

/// Shows the panel where the user left it, if the setting allows.
fn show(app: &AppHandle<Wry>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let recording = crate::features::settings::current(&app).await.recording;
        if !recording.show_panel {
            return;
        }
        let position = recording.panel_position.unwrap_or(DEFAULT_POSITION);
        if let Some(host) = app.try_state::<Arc<PanelHost>>() {
            host.set_edge(position.edge);
        }
        let handle = app.clone();
        let shown = app.run_on_main_thread(move || {
            let at = screen_origin(&handle, position);
            if let Err(error) = platform::show(&handle, at) {
                log::error!(target: "recording", "could not show the record panel: {error}");
                return;
            }
            start_driver(&handle);
        });
        if let Err(error) = shown {
            log::error!(target: "recording", "could not reach the main thread: {error}");
        }
    });
}

/// Hides the panel and stops its driver.
fn hide(app: &AppHandle<Wry>) {
    if let Some(host) = app.try_state::<Arc<PanelHost>>() {
        drop(
            host.driver
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take(),
        );
    }
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || platform::hide(&handle)) {
        log::error!(target: "recording", "could not reach the main thread: {error}");
    }
}

fn start_driver(app: &AppHandle<Wry>) {
    let Some(host) = app.try_state::<Arc<PanelHost>>() else {
        return;
    };
    let mut driver = host.driver.lock().unwrap_or_else(PoisonError::into_inner);
    if driver.is_none() {
        *driver = Driver::start(app.clone(), Arc::clone(&host)).ok();
    }
}

/// The window's top-left for `position` on the screen it is (or will be) on.
fn screen_origin(app: &AppHandle<Wry>, position: PanelPosition) -> Option<LogicalPosition<f64>> {
    let screen = work_area(app)?;
    let (x, y) = origin(position, FRAME, screen);
    Some(LogicalPosition::new(x, y))
}

/// The work area (below the menu bar, beside the Dock) of the screen the
/// panel is on, or of the primary screen.
pub(crate) fn work_area(app: &AppHandle<Wry>) -> Option<Rect> {
    let on_panel = app
        .get_webview_window(PANEL_LABEL)
        .and_then(|window| window.current_monitor().ok().flatten());
    let monitor = on_panel.or_else(|| app.primary_monitor().ok().flatten())?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    Some(Rect {
        x: f64::from(area.position.x) / scale,
        y: f64::from(area.position.y) / scale,
        width: f64::from(area.size.width) / scale,
        height: f64::from(area.size.height) / scale,
    })
}

#[cfg(target_os = "macos")]
#[path = "panel/panel_mac.rs"]
mod platform;

#[cfg(not(target_os = "macos"))]
#[path = "panel/panel_window.rs"]
mod platform;
