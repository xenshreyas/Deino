//! The record panel's driver: a thread that runs while the panel shows,
//! about [`FRAME_RATE`] times a second.
//!
//! - **Pointer.** Where the pointer is against the pill's rectangle: over
//!   it, the window takes the mouse and the page hears `panel-pointer`
//!   (hover); elsewhere, clicks pass through to the app below.
//! - **Levels.** Each track's loudest sample since the last frame, to the
//!   panel only (Handy's throttled `emit_to`), while capture runs.
//! - **Drops.** A window that moved and then stayed put for [`SETTLE_FRAMES`] was
//!   dragged and dropped: it snaps to the nearer edge, the page hears
//!   `panel-side`, and the position is saved.

use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_core::settings::{PanelEdge, PanelPosition, SettingsService};
use deino_recording::{RecordingPhase, RecordingService};
use tauri::{AppHandle, LogicalPosition, Manager, PhysicalPosition, WebviewWindow, Wry};
use tauri_specta::Event;

use super::placement::{origin, snap};
use super::{FRAME, PANEL_LABEL, PanelHost, work_area};
use crate::features::recording::events::{AudioLevel, PanelPointer, PanelSide};

/// Frames a second: the meter's rate, and how often the pointer is checked.
const FRAME_RATE: u32 = 30;

/// Frames a moved window must stay still to count as dropped (400 ms).
const SETTLE_FRAMES: u32 = 12;

/// Extra points around the pill that still count as over it.
const SLOP: f64 = 2.0;

/// A running driver; dropping it stops the thread.
pub(super) struct Driver {
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Driver {
    /// Starts driving the panel.
    pub(super) fn start(app: AppHandle<Wry>, host: Arc<PanelHost>) -> std::io::Result<Self> {
        let (stop, stopped) = channel::<()>();
        let frame = Duration::from_secs(1) / FRAME_RATE;
        let thread = std::thread::Builder::new()
            .name("deino-record-panel".into())
            .spawn(move || {
                let mut state = Frames::default();
                while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(frame) {
                    state.step(&app, &host);
                }
            })?;
        Ok(Self {
            stop: Some(stop),
            thread: Some(thread),
        })
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        drop(self.stop.take());
        let joined = self.thread.take().map(JoinHandle::join);
        if matches!(joined, Some(Err(_))) {
            log::error!(target: "recording", "the record panel driver panicked");
        }
    }
}

/// What the driver remembers between frames.
#[derive(Default)]
struct Frames {
    inside: Option<bool>,
    /// Where the window was last frame, and for how many frames it stayed.
    last_position: Option<PhysicalPosition<i32>>,
    still: u32,
    /// Where the driver itself last put the window.
    placed: Option<PhysicalPosition<i32>>,
}

impl Frames {
    fn step(&mut self, app: &AppHandle<Wry>, host: &PanelHost) {
        let Some(window) = app.get_webview_window(PANEL_LABEL) else {
            return;
        };
        self.track_pointer(app, &window, host);
        send_levels(app);
        self.snap_after_drop(app, &window, host);
    }

    /// Lets the pill take the mouse while the pointer is over it.
    fn track_pointer(
        &mut self,
        app: &AppHandle<Wry>,
        window: &WebviewWindow<Wry>,
        host: &PanelHost,
    ) {
        let inside = pointer_inside(app, window, host).unwrap_or(false);
        if self.inside == Some(inside) {
            return;
        }
        self.inside = Some(inside);
        if let Err(error) = window.set_ignore_cursor_events(!inside) {
            log::debug!(target: "recording", "could not switch the panel's click-through: {error}");
        }
        PanelPointer { inside }.emit_to(app, PANEL_LABEL).ok();
    }

    /// Snaps a dragged window to the nearer edge once it settles.
    fn snap_after_drop(
        &mut self,
        app: &AppHandle<Wry>,
        window: &WebviewWindow<Wry>,
        host: &PanelHost,
    ) {
        let Ok(position) = window.outer_position() else {
            return;
        };
        if self.last_position != Some(position) {
            // The first position is where the panel was shown: ours.
            self.placed = self.placed.or(Some(position));
            self.last_position = Some(position);
            self.still = 0;
            return;
        }
        self.still += 1;
        if self.still == SETTLE_FRAMES && self.placed != Some(position) {
            self.placed = place(app, window, position).map(|(placed, edge)| {
                host.set_edge(edge);
                placed
            });
        }
    }
}

/// Whether the pointer is over the pill.
fn pointer_inside(
    app: &AppHandle<Wry>,
    window: &WebviewWindow<Wry>,
    host: &PanelHost,
) -> Option<bool> {
    let region = host.region()?;
    let cursor = app.cursor_position().ok()?;
    let at = window.outer_position().ok()?;
    let scale = window.scale_factor().ok()?;
    let x = (cursor.x - f64::from(at.x)) / scale;
    let y = (cursor.y - f64::from(at.y)) / scale;
    Some(region.contains((x, y), SLOP))
}

/// Sends the levels to the panel while capture runs.
fn send_levels(app: &AppHandle<Wry>) {
    let Some(recorder) = app.try_state::<Arc<RecordingService>>() else {
        return;
    };
    if recorder.state().phase != RecordingPhase::Recording {
        return;
    }
    if let Some(levels) = recorder.levels() {
        AudioLevel { levels }.emit_to(app, PANEL_LABEL).ok();
    }
}

/// Snaps the window at `position` to its edge, saves where, and returns
/// where it was put and against which edge.
fn place(
    app: &AppHandle<Wry>,
    window: &WebviewWindow<Wry>,
    position: PhysicalPosition<i32>,
) -> Option<(PhysicalPosition<i32>, PanelEdge)> {
    let scale = window.scale_factor().ok()?;
    let screen = work_area(app)?;
    let logical = (f64::from(position.x) / scale, f64::from(position.y) / scale);
    let snapped = snap(logical, FRAME, screen);
    let (x, y) = origin(snapped, FRAME, screen);
    window.set_position(LogicalPosition::new(x, y)).ok()?;
    PanelSide { edge: snapped.edge }
        .emit_to(app, PANEL_LABEL)
        .ok();
    remember(app, snapped);
    Some((window.outer_position().ok()?, snapped.edge))
}

/// Saves where the panel was left.
fn remember(app: &AppHandle<Wry>, position: PanelPosition) {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        return;
    };
    let settings = Arc::clone(&settings);
    tauri::async_runtime::spawn(async move {
        let saved = settings
            .update(move |settings| settings.recording.panel_position = Some(position))
            .await;
        if let Err(error) = saved {
            log::warn!(target: "recording", "could not save the record panel's position: {error}");
        }
    });
}
