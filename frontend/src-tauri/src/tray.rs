//! The menu bar (CAP-10): Deino's icon with the time left in the recorded
//! meeting or a countdown to the next one beside it, and a menu with the
//! recording's controls and today's meetings (Join / Record / Open).
//!
//! - [`view`], what to show, as plain data (tested);
//! - [`menu`], that data as a native menu, and what its items do.
//!
//! The menu is rebuilt when the recorder's phase changes and when the
//! calendar syncs; the title follows the clock through a light ticker that
//! re-reads both every [`TITLE_TICK`].

mod menu;
mod view;

use std::sync::Arc;
use std::time::Duration;

use chrono::{Days, Utc};
use deino_core::calendar::{CalendarService, TimeRange};
use deino_domain::{LocalZone, SystemZone};
use deino_recording::{RecordingPhase, RecordingService};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

/// The tray icon's ID.
const TRAY_ID: &str = "main-tray";

/// How often the title re-reads the clock (it shows whole minutes).
const TITLE_TICK: Duration = Duration::from_secs(20);

/// Creates the menu bar item and starts its title ticker.
pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = menu::build(app, &menu::Contents::default())?;
    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(crate::APP_NAME)
        .on_menu_event(|app, event| menu::handle(app, event.id.as_ref()));
    with_icon(builder, app).build(app)?;
    refresh(app);
    start_ticker(app.clone());
    Ok(())
}

/// The menu bar mark (BRAND-01): black plus alpha, 36 px tall, which the
/// tray shows 18 pt tall (crisp on Retina).
#[cfg(target_os = "macos")]
const TEMPLATE_ICON: tauri::image::Image<'static> =
    tauri::include_image!("icons/tray/tray-icon@2x.png");

/// On macOS the mark as a template image, which the system tints for light
/// and dark menu bars (and while the menu is open).
#[cfg(target_os = "macos")]
fn with_icon<R: Runtime>(builder: TrayIconBuilder<R>, _app: &AppHandle<R>) -> TrayIconBuilder<R> {
    builder.icon(TEMPLATE_ICON).icon_as_template(true)
}

/// Elsewhere the full-colour app icon: a black template would vanish on a
/// dark taskbar.
#[cfg(not(target_os = "macos"))]
fn with_icon<R: Runtime>(builder: TrayIconBuilder<R>, app: &AppHandle<R>) -> TrayIconBuilder<R> {
    match app.default_window_icon() {
        Some(icon) => builder.icon(icon.clone()),
        None => builder,
    }
}

/// Rebuilds the menu and the title from the recorder and today's calendar.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    update(app, true);
}

/// Updates the title only (the ticker: an open menu stays open).
fn refresh_title<R: Runtime>(app: &AppHandle<R>) {
    update(app, false);
}

fn update<R: Runtime>(app: &AppHandle<R>, with_menu: bool) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let contents = contents(&app).await;
        let Some(tray) = app.tray_by_id(TRAY_ID) else {
            return;
        };
        let menu = if with_menu {
            menu::build(&app, &contents).map(Some)
        } else {
            Ok(None)
        };
        let shown = menu
            .and_then(|menu| menu.map_or(Ok(()), |menu| tray.set_menu(Some(menu))))
            .and_then(|()| tray.set_title(contents.title.as_deref()));
        if let Err(error) = shown {
            log::warn!(target: "tray", "could not update the menu bar: {error}");
        }
    });
}

/// Brings the main window forward and focuses it.
pub(crate) fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return log::warn!(target: "tray", "there is no main window");
    };
    let shown = window
        .unminimize()
        .and_then(|()| window.show())
        .and_then(|()| window.set_focus());
    if let Err(error) = shown {
        log::warn!(target: "tray", "could not bring the main window forward: {error}");
    }
}

/// Everything the menu shows now.
async fn contents<R: Runtime>(app: &AppHandle<R>) -> menu::Contents {
    let now = now();
    let today = today(app).await;
    let recording = recording(app, &today);
    let title = view::title(recording.as_ref(), &today, now);
    menu::Contents {
        phase: app
            .try_state::<Arc<RecordingService>>()
            .map_or(RecordingPhase::Idle, |recorder| recorder.state().phase),
        recording,
        today,
        title,
        now,
    }
}

/// The recording, with its event's end for "Xm left".
fn recording<R: Runtime>(app: &AppHandle<R>, today: &[view::Meeting]) -> Option<view::Recording> {
    let state = app.try_state::<Arc<RecordingService>>()?.state();
    if !state.is_running() {
        return None;
    }
    let ends_at = state
        .calendar_event_id
        .and_then(|id| today.iter().find(|meeting| meeting.id == id))
        .map(|meeting| meeting.ends_at);
    Some(view::Recording {
        title: state.title.unwrap_or_default(),
        paused: state.phase == RecordingPhase::Paused,
        started_at: state.started_at?,
        ends_at,
    })
}

/// Today's meetings (local day), without all-day ones, by start.
async fn today<R: Runtime>(app: &AppHandle<R>) -> Vec<view::Meeting> {
    let Some(calendar) = app.try_state::<Arc<CalendarService>>() else {
        return Vec::new();
    };
    let zone = SystemZone;
    let day = zone.day_of(now());
    let Some(next) = day.checked_add_days(Days::new(1)) else {
        return Vec::new();
    };
    let range = TimeRange {
        start: zone.start_of(day),
        end: zone.start_of(next),
    };
    let events = calendar.upcoming(range).await.unwrap_or_default();
    events
        .into_iter()
        .filter(|event| !event.is_all_day)
        .map(|event| view::Meeting {
            id: event.id,
            title: event.title,
            starts_at: event.start_at,
            ends_at: event.end_at,
            join_url: event.conference_url,
        })
        .collect()
}

/// Re-reads the clock for the title every [`TITLE_TICK`], for the app's life.
fn start_ticker<R: Runtime>(app: AppHandle<R>) {
    let ticker = std::thread::Builder::new()
        .name("deino-menu-bar".into())
        .spawn(move || {
            let (_keep, timer) = std::sync::mpsc::channel::<()>();
            while timer.recv_timeout(TITLE_TICK).is_err() {
                refresh_title(&app);
            }
        });
    if let Err(error) = ticker {
        log::warn!(target: "tray", "the menu bar title will not tick: {error}");
    }
}

/// The wall clock (the sanctioned reader, `SystemClock`).
fn now() -> chrono::DateTime<Utc> {
    use deino_domain::Clock;
    deino_domain::SystemClock.now()
}
