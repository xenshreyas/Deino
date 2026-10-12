//! The meeting ticker (ORG-02): every [`TICK`] it looks at the events
//! around now and
//!
//! - shows the reminder window ([`ReminderWindow`]) for a meeting starting
//!   within a minute ([`ReminderClock`](super::notify::ReminderClock)), and
//!   closes it a minute later; Deino's Do Not Disturb skips it;
//! - starts recording an event whose note the user has open
//!   ([`OpenNote`]) when its start time comes and its auto-record rules say
//!   so ([`StartTrigger::StartTime`]), through [`JoinService`] like Home's
//!   Join, unless something is already recording.
//!
//! The bookkeeping is the pure [`TickState`]; the app spawns
//! [`MeetingTicker::run`] once and cancels it at exit, like the
//! [`SyncScheduler`](super::SyncScheduler).

mod plan;

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use deino_domain::{CalendarEventId, Clock};
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

pub use plan::{REMINDER_OPEN_FOR, START_GRACE, TickInput, TickPlan, TickState};

use super::join::{JoinMode, JoinService};
use super::rules::{AutoRecordService, StartTrigger};
use super::{CalendarEvent, CalendarService};
use crate::settings::SettingsService;

/// How often the ticker looks.
pub const TICK: Duration = Duration::from_secs(15);

/// The small "starting in 1 minute" window.
pub trait ReminderWindow: Send + Sync {
    /// Shows the window for `event`, replacing any other.
    fn show(&self, event: &CalendarEvent);
    /// Closes the window, if it is up.
    fn close(&self);
}

/// Whether Deino is recording.
pub trait RecorderBusy: Send + Sync {
    /// True while a recording runs (or starts, or stops).
    fn is_recording(&self) -> bool;
}

/// The calendar event whose note is on screen, as the UI reports it.
#[derive(Debug, Default)]
pub struct OpenNote(Mutex<Option<CalendarEventId>>);

impl OpenNote {
    /// The UI opened `event`'s note, or closed it (`None`).
    pub fn set(&self, event: Option<CalendarEventId>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = event;
    }

    /// The event whose note is open.
    pub fn get(&self) -> Option<CalendarEventId> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What the ticker reads and acts through.
pub struct TickerDeps {
    /// The events around now.
    pub calendar: Arc<CalendarService>,
    /// The auto-record rules.
    pub rules: Arc<AutoRecordService>,
    /// Starts recording an event.
    pub joins: Arc<JoinService>,
    /// The reminder and Do Not Disturb settings.
    pub settings: Arc<SettingsService>,
    /// The reminder window.
    pub window: Arc<dyn ReminderWindow>,
    /// Whether something is recording already.
    pub recorder: Arc<dyn RecorderBusy>,
    /// The note on screen.
    pub open_note: Arc<OpenNote>,
    /// The time.
    pub clock: Arc<dyn Clock>,
}

/// Shows reminders and starts recordings on time.
pub struct MeetingTicker {
    deps: TickerDeps,
    state: Mutex<TickState>,
}

impl MeetingTicker {
    /// A ticker that has reminded nobody yet.
    pub fn new(deps: TickerDeps) -> Self {
        Self {
            deps,
            state: Mutex::new(TickState::default()),
        }
    }

    /// Ticks every [`TICK`] until `cancel` fires.
    pub async fn run(self: Arc<Self>, cancel: CancellationToken) {
        let mut interval = tokio::time::interval(TICK);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                () = cancel.cancelled() => return,
                _ = interval.tick() => self.tick().await,
            }
        }
    }

    /// One look at the events around now.
    pub async fn tick(&self) {
        let now = self.deps.clock.now();
        let events = self.deps.calendar.events_near(now);
        let settings = self.deps.settings.get().await;
        let plan = self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .plan(&TickInput {
                events: &events,
                now,
                reminders: &settings.calendar.reminders,
                muted: settings.notifications.manual_dnd_mode,
                open_note: self.deps.open_note.get(),
            });
        if plan.close {
            self.deps.window.close();
        }
        if let Some(event) = &plan.show {
            self.deps.window.show(event);
        }
        if let Some(event) = plan.start {
            self.start_on_time(event.id).await;
        }
    }

    /// Records event `id` if its rules start it at its start time and
    /// nothing is recording yet.
    async fn start_on_time(&self, id: CalendarEventId) {
        if self.deps.recorder.is_recording() {
            return;
        }
        let trigger = StartTrigger::StartTime { note_open: true };
        match self.deps.rules.starts_on(id, trigger).await {
            Ok(true) => {
                log::info!(target: "calendar", "event {id} started with its note open; recording");
                if let Err(error) = self.deps.joins.join(id, JoinMode::RecordOnly).await {
                    log::warn!(target: "calendar", "event {id} did not start recording: {error}");
                }
            }
            Ok(false) => {}
            Err(error) => log::warn!(target: "calendar", "event {id}: {error}"),
        }
    }
}

#[cfg(test)]
mod ticker_tests;
