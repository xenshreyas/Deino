//! The morning reminder in the app: [`SystemNotifier`] shows the core's
//! `Notice` as a system notification (`tauri-plugin-notification`), and
//! [`start_reminders`] runs `DailyReminder::tick` once a minute for the
//! app's lifetime. The rules (09:00, catch-up until noon, once a day, the
//! settings switch) are `deino_core::action_items::DailyReminder`'s.

use std::sync::Arc;
use std::time::Duration;

use deino_core::action_items::{DailyReminder, Notice, Notifier};
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

/// How often the reminder checks the clock. A minute keeps it on time after
/// sleep and time zone changes without measurable cost.
const TICK: Duration = Duration::from_mins(1);

/// Shows notices through the system's notification center.
pub(crate) struct SystemNotifier<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> SystemNotifier<R> {
    /// A notifier that shows through `app`.
    pub(crate) fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> Notifier for SystemNotifier<R> {
    fn notify(&self, notice: &Notice) {
        let shown = self
            .app
            .notification()
            .builder()
            .title(&notice.title)
            .body(&notice.body)
            .show();
        // The notice names the user's items, so only the failure is logged.
        if let Err(error) = shown {
            log::warn!(target: "action_items", "could not show the action item reminder: {error}");
        }
    }
}

/// Ticks `reminder` once a minute until the app exits. Owned for the app's
/// lifetime: the task ends with the async runtime at exit.
pub(crate) fn start_reminders(reminder: Arc<DailyReminder>) {
    tauri::async_runtime::spawn(async move {
        let mut ticks = tokio::time::interval(TICK);
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticks.tick().await;
            match reminder.tick().await {
                Ok(Some(_)) => log::info!(target: "action_items", "showed the morning reminder"),
                Ok(None) => {}
                Err(error) => {
                    log::warn!(target: "action_items", "the morning reminder failed: {error}");
                }
            }
        }
    });
}
