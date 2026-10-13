//! The meeting reminder window (ORG-02): a small, always-on-top card in the
//! top-right corner, "starting in 1 minute", with Join & record, Record only
//! and dismiss. It never takes focus from what the user is doing: on macOS a
//! non-activating `NSPanel` at status level on every Space (CAP-06's prompt
//! pattern), elsewhere a plain always-on-top window shown unfocused.
//!
//! [`ReminderCard`] is the meeting ticker's `ReminderWindow`: it remembers
//! which event the card is for and drives [`TauriReminderWindow`], which
//! tells the page ([`ReminderShown`]) and shows or hides the window. The
//! page (`meeting-reminder.html`) reads the event through
//! `calendar|reminder`.

use std::sync::{Arc, Mutex, PoisonError};

use deino_core::calendar::CalendarEvent;
use deino_core::calendar::ticker::ReminderWindow;
use deino_domain::CalendarEventId;
use tauri::{AppHandle, LogicalPosition, Manager, Wry};
use tauri_specta::Event;

use super::events::ReminderShown;

/// The window's label; `capabilities/meeting-reminder.json` grants it.
const REMINDER_LABEL: &str = "meeting_reminder";

/// The page the window loads (a Vite entry next to `index.html`).
const REMINDER_PAGE: &str = "meeting-reminder.html";

/// The window, in logical pixels. The card inside leaves room for its shadow.
const WIDTH: f64 = 376.0;
const HEIGHT: f64 = 176.0;

/// The gap below the menu bar, minus the shadow room. There is none on the
/// right: the window reaches the screen's edge so the card can slide in from
/// off-screen (its page keeps the card inset from that edge).
const INSET: f64 = 8.0;

/// The card's state: which event it is for. The meeting ticker shows and
/// closes the window through it; the page and its buttons read and close it
/// through the commands.
pub struct ReminderCard {
    window: Arc<dyn ReminderWindow>,
    current: Mutex<Option<CalendarEventId>>,
}

impl ReminderCard {
    /// A card over `window`, showing nothing yet.
    pub fn new(window: Arc<dyn ReminderWindow>) -> Self {
        Self {
            window,
            current: Mutex::new(None),
        }
    }

    /// The event the card is for, while it is up.
    pub fn current(&self) -> Option<CalendarEventId> {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set(&self, event: Option<CalendarEventId>) {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = event;
    }
}

impl ReminderWindow for ReminderCard {
    fn show(&self, event: &CalendarEvent) {
        self.set(Some(event.id));
        self.window.show(event);
    }

    fn close(&self) {
        self.set(None);
        self.window.close();
    }
}

/// The window itself, over the app.
pub struct TauriReminderWindow {
    app: AppHandle<Wry>,
}

impl TauriReminderWindow {
    /// The window of `app`, built on the first reminder.
    pub fn new(app: AppHandle<Wry>) -> Self {
        Self { app }
    }
}

impl ReminderWindow for TauriReminderWindow {
    fn show(&self, event: &CalendarEvent) {
        log::info!(target: "calendar", "reminding of event {}", event.id);
        if let Err(error) = (ReminderShown { event_id: event.id }).emit(&self.app) {
            log::warn!(target: "calendar", "could not tell the reminder page: {error}");
        }
        on_main_thread(&self.app, |app| {
            if let Err(error) = platform::show(app, top_right(app)) {
                log::error!(target: "calendar", "could not show the reminder: {error}");
            }
        });
    }

    fn close(&self) {
        on_main_thread(&self.app, platform::hide);
    }
}

/// Runs `act` on the main thread, as `AppKit` requires.
fn on_main_thread(app: &AppHandle<Wry>, act: impl FnOnce(&AppHandle<Wry>) + Send + 'static) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || act(&handle)) {
        log::error!(target: "calendar", "could not reach the main thread: {error}");
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
    let (Some(window), Some(position)) = (app.get_webview_window(REMINDER_LABEL), position) else {
        return;
    };
    if let Err(error) = window.set_position(position) {
        log::debug!(target: "calendar", "could not move the reminder: {error}");
    }
}

#[cfg(target_os = "macos")]
#[path = "reminder_panel.rs"]
mod platform;

#[cfg(not(target_os = "macos"))]
#[path = "reminder_plain.rs"]
mod platform;
