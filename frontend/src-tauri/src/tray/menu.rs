//! The menu bar's menu, and what its items do.
//!
//! ```text
//! ● Weekly sync — 12m left          (while recording)
//!   Pause · Stop · Open notes
//! ─────────
//! Today
//!   09:30  Design review   ▸ Join call · Record · Open in Deino
//!   14:00  Board prep      ▸ …
//! ─────────
//! Start recording   ⌘⇧R           (while idle)
//! Open Deino · Settings…
//! ─────────
//! Quit Deino
//! ```

use std::sync::Arc;

use chrono::{DateTime, Utc};
use deino_core::calendar::join::{JoinMode, JoinService};
use deino_domain::{CalendarEventId, LocalZone, SystemZone};
use deino_recording::{RecordingPhase, RecordingService, StartRequest};
use tauri::menu::{IsMenuItem, Menu, MenuBuilder, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use super::view::{self, Meeting, Recording};
use crate::features::recording::OpenLiveMeeting;

/// Everything the menu shows.
#[derive(Clone, Debug)]
pub(super) struct Contents {
    pub phase: RecordingPhase,
    pub recording: Option<Recording>,
    pub today: Vec<Meeting>,
    pub title: Option<String>,
    pub now: DateTime<Utc>,
}

impl Default for Contents {
    fn default() -> Self {
        Self {
            phase: RecordingPhase::Idle,
            recording: None,
            today: Vec::new(),
            title: None,
            now: DateTime::<Utc>::MIN_UTC,
        }
    }
}

/// The menu for `contents`.
pub(super) fn build<R: Runtime>(app: &AppHandle<R>, contents: &Contents) -> tauri::Result<Menu<R>> {
    let mut builder = MenuBuilder::new(app);
    for item in recording_items(app, contents)? {
        builder = builder.item(&*item);
    }
    builder = builder.separator();
    for item in agenda_items(app, contents)? {
        builder = builder.item(&*item);
    }
    builder = builder
        .separator()
        .item(&MenuItem::with_id(
            app,
            "open",
            "Open Deino",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "settings",
            "Settings…",
            true,
            None::<&str>,
        )?);
    if crate::updater::AUTO_UPDATE_ENABLED {
        builder = builder.text("check_updates", "Check for Updates");
    }
    builder
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some("Quit Deino"))?)
        .build()
}

type Items<R> = Vec<Box<dyn IsMenuItem<R>>>;

/// The recording's line and controls, or "Start recording".
fn recording_items<R: Runtime>(app: &AppHandle<R>, contents: &Contents) -> tauri::Result<Items<R>> {
    let item = |id: &str, text: &str, enabled: bool| -> tauri::Result<Box<dyn IsMenuItem<R>>> {
        Ok(Box::new(MenuItem::with_id(
            app,
            id,
            text,
            enabled,
            None::<&str>,
        )?))
    };
    let Some(recording) = &contents.recording else {
        return Ok(match contents.phase {
            RecordingPhase::Starting => vec![item("starting", "Starting…", false)?],
            RecordingPhase::Stopping => vec![item("stopping", "Saving…", false)?],
            _ => vec![item("recording:start", "Start Recording", true)?],
        });
    };
    let status = view::status(recording, contents.now);
    let heading = format!("● {} — {status}", view::short(&recording.title));
    let toggle = if recording.paused {
        item("recording:resume", "Resume", true)?
    } else {
        item("recording:pause", "Pause", true)?
    };
    Ok(vec![
        item("recording:heading", &heading, false)?,
        toggle,
        item("recording:stop", "Stop Recording", true)?,
        item("recording:notes", "Open Notes", true)?,
    ])
}

/// "Today" and the meetings still to come or under way.
fn agenda_items<R: Runtime>(app: &AppHandle<R>, contents: &Contents) -> tauri::Result<Items<R>> {
    let ahead: Vec<&Meeting> = contents
        .today
        .iter()
        .filter(|meeting| meeting.ends_at > contents.now)
        .collect();
    let heading = if ahead.is_empty() {
        "No more meetings today"
    } else {
        "Today"
    };
    let mut items: Items<R> = vec![Box::new(MenuItem::new(app, heading, false, None::<&str>)?)];
    for meeting in ahead {
        items.push(Box::new(meeting_menu(app, meeting)?));
    }
    Ok(items)
}

/// One meeting: its time and title, with Join, Record and Open.
fn meeting_menu<R: Runtime>(app: &AppHandle<R>, meeting: &Meeting) -> tauri::Result<Submenu<R>> {
    let at = SystemZone.local(meeting.starts_at).format("%H:%M");
    let label = format!("{at}  {}", view::short(&meeting.title));
    let id = |action: &str| format!("meeting:{}:{action}", meeting.id);
    let join = MenuItem::with_id(
        app,
        id("join"),
        "Join Call",
        meeting.join_url.is_some(),
        None::<&str>,
    )?;
    let record = MenuItem::with_id(app, id("record"), "Record", true, None::<&str>)?;
    let open = MenuItem::with_id(app, id("open"), "Open in Deino", true, None::<&str>)?;
    Submenu::with_items(app, label, true, &[&join, &record, &open])
}

/// Does what menu item `id` says.
pub(super) fn handle<R: Runtime>(app: &AppHandle<R>, id: &str) {
    if let Some(rest) = id.strip_prefix("meeting:") {
        return meeting_action(app, rest);
    }
    match id {
        "recording:start" => start(app, StartRequest::default()),
        "recording:pause" => control(app, |recorder| recorder.pause().map(drop)),
        "recording:resume" => control(app, |recorder| recorder.resume().map(drop)),
        "recording:stop" => stop(app),
        "recording:notes" => open_notes(app),
        "open" => super::focus_main_window(app),
        "settings" => open_settings(app),
        "check_updates" => check_updates(app),
        _ => {}
    }
}

fn recorder<R: Runtime>(app: &AppHandle<R>) -> Option<Arc<RecordingService>> {
    app.try_state::<Arc<RecordingService>>()
        .map(|state| Arc::clone(&state))
}

fn start<R: Runtime>(app: &AppHandle<R>, request: StartRequest) {
    let Some(recorder) = recorder(app) else {
        return log::warn!(target: "tray", "the recorder is not ready yet");
    };
    super::focus_main_window(app);
    tauri::async_runtime::spawn(async move {
        if let Err(error) = recorder.start(request).await {
            log::warn!(target: "tray", "recording did not start: {error}");
        }
    });
}

fn stop<R: Runtime>(app: &AppHandle<R>) {
    let Some(recorder) = recorder(app) else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        if let Err(error) = recorder.stop().await {
            log::warn!(target: "tray", "recording did not stop: {error}");
        }
    });
}

fn control<R: Runtime>(
    app: &AppHandle<R>,
    act: impl FnOnce(&RecordingService) -> Result<(), deino_recording::RecordingError>,
) {
    if let Some(Err(error)) = recorder(app).map(|recorder| act(&recorder)) {
        log::warn!(target: "tray", "the recording control failed: {error}");
    }
}

/// Brings the main window forward on the live meeting.
fn open_notes<R: Runtime>(app: &AppHandle<R>) {
    super::focus_main_window(app);
    let Some(meeting_id) = recorder(app).and_then(|recorder| recorder.state().meeting_id) else {
        return;
    };
    OpenLiveMeeting { meeting_id }.emit(app).ok();
}

fn open_settings<R: Runtime>(app: &AppHandle<R>) {
    super::focus_main_window(app);
    if let Some(window) = app.get_webview_window("main") {
        window.eval("window.location.assign('/settings')").ok();
    }
}

fn check_updates<R: Runtime>(app: &AppHandle<R>) {
    super::focus_main_window(app);
    if let Some(window) = app.get_webview_window("main") {
        window
            .eval("window.dispatchEvent(new CustomEvent('check-updates-from-tray'))")
            .ok();
    }
}

/// `<event id>:<join|record|open>` on one of today's meetings. Join and
/// Record go through `JoinService`, the path Home's Join and the reminder
/// window take (ORG-02).
fn meeting_action<R: Runtime>(app: &AppHandle<R>, rest: &str) {
    let Some((id, action)) = rest.rsplit_once(':') else {
        return;
    };
    let Ok(id) = id.parse::<CalendarEventId>() else {
        return;
    };
    let mode = match action {
        "join" => JoinMode::JoinOnly,
        "record" => JoinMode::RecordOnly,
        _ => return super::focus_main_window(app),
    };
    let Some(joins) = app.try_state::<Arc<JoinService>>().map(|j| Arc::clone(&j)) else {
        return log::warn!(target: "tray", "Join is not ready yet");
    };
    if mode == JoinMode::RecordOnly {
        super::focus_main_window(app);
    }
    tauri::async_runtime::spawn(async move {
        if let Err(error) = joins.join(id, mode).await {
            log::warn!(target: "tray", "event {id}: {error}");
        }
    });
}
