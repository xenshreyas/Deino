//! The app's side of auto-stop (CAP-07): `deino-detect`'s ports over the
//! live recorder, the synced calendar, the settings and Tauri.
//!
//! - [`RecorderProbe`], the running recording, what its lanes heard and
//!   whether its disk is full;
//! - [`SyncedWindows`], the calendar's in-memory events around now;
//! - [`SettingsAutoStop`], the `Settings::recording.auto_stop` section;
//! - [`TauriAutoStopActions`], the notice and offer as `auto-stop-changed`
//!   (the notice also as a top-right card), and the stop and the split on
//!   the recorder, in the background.
//!
//! Each looks its service up in Tauri state on every call: the recorder
//! and the calendar are attached with the database, after auto-stop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use deino_core::calendar::{CalendarEvent, CalendarService};
use deino_core::meetings::MeetingLink;
use deino_core::settings::SettingsService;
use deino_detect::autostop::{
    AutoStopActions, AutoStopConfig, AutoStopSettings, AutoStopState, CalendarWindows, EventWindow,
    RecordingProbe, RecordingView, SpeechTotals, StopReason,
};
use deino_domain::{Millis, Rsvp};
use deino_recording::{IssueKind, RecordingPhase, RecordingService, StartRequest};
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use super::events::AutoStopChanged;

/// The live recorder in `app`'s state, once it is attached.
fn recorder<R: Runtime>(app: &AppHandle<R>) -> Option<Arc<RecordingService>> {
    app.try_state::<Arc<RecordingService>>()
        .map(|state| Arc::clone(&state))
}

/// The running recording, as auto-stop sees it.
pub struct RecorderProbe<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> RecordingProbe for RecorderProbe<R> {
    fn recording(&self) -> Option<RecordingView> {
        let recorder = recorder(&self.0)?;
        let state = recorder.state();
        if !state.is_running() {
            return None;
        }
        let speech = recorder
            .activity()
            .map_or(UNHEARD, |activity| SpeechTotals {
                mic: millis(activity.me.voiced.0),
                system: millis(activity.them.voiced.0),
                mic_speaking: activity.me.speaking,
                system_speaking: activity.them.speaking,
            });
        Some(RecordingView {
            meeting: state.meeting_id.clone()?,
            started_at: state.started_at?,
            paused: state.phase == RecordingPhase::Paused,
            speech,
            disk_full: state.issues.iter().any(|i| i.kind == IssueKind::DiskFull),
            event: state.calendar_event_id,
        })
    }
}

/// What a record-only recording reports (CAP-08: no lane listens, so who
/// speaks is unknown): someone always speaking on the microphone. The
/// silence rules (no speech, the calendar's quiet) then never end it, and
/// the call app releasing the microphone still does, since only speech on
/// the call delays that.
const UNHEARD: SpeechTotals = SpeechTotals {
    mic: Millis(0),
    system: Millis(0),
    mic_speaking: true,
    system_speaking: false,
};

/// A lane total in stored milliseconds (saturating past 49 days).
fn millis(total: u64) -> Millis {
    Millis(u32::try_from(total).unwrap_or(u32::MAX))
}

/// The synced calendar's events around now.
pub struct SyncedWindows<R: Runtime> {
    app: AppHandle<R>,
    settings: Arc<SettingsService>,
}

impl<R: Runtime> SyncedWindows<R> {
    /// Windows over `app`'s calendar, leaving out the user's own addresses.
    pub fn new(app: AppHandle<R>, settings: Arc<SettingsService>) -> Self {
        Self { app, settings }
    }
}

impl<R: Runtime> CalendarWindows for SyncedWindows<R> {
    fn events_near(&self, now: DateTime<Utc>) -> Vec<EventWindow> {
        let Some(calendar) = self.app.try_state::<Arc<CalendarService>>() else {
            return Vec::new();
        };
        // Called on the auto-stop thread, outside the async runtime.
        let profile = tauri::async_runtime::block_on(self.settings.get()).profile;
        let mine: Vec<String> = profile
            .emails
            .iter()
            .map(|email| email.as_str().to_lowercase())
            .collect();
        calendar
            .events_near(now)
            .iter()
            .filter(|event| !event.is_all_day && event.my_rsvp != Some(Rsvp::Declined))
            .map(|event| window(event, &mine))
            .collect()
    }
}

/// `event` as a window, its invitees lowercased and without `mine`.
pub(crate) fn window(event: &CalendarEvent, mine: &[String]) -> EventWindow {
    let attendees = event
        .attendees
        .iter()
        .filter_map(|attendee| attendee.email.as_deref())
        .map(str::to_lowercase)
        .filter(|email| !mine.contains(email))
        .collect();
    EventWindow {
        id: event.id,
        title: event.title.clone(),
        starts_at: event.start_at,
        ends_at: event.end_at,
        series_id: event.series_id.clone(),
        attendees,
    }
}

/// `Settings::recording.auto_stop`, read on every tick.
pub struct SettingsAutoStop(pub Arc<SettingsService>);

impl AutoStopSettings for SettingsAutoStop {
    fn config(&self) -> AutoStopConfig {
        // Called on the auto-stop thread, outside the async runtime.
        let options = tauri::async_runtime::block_on(self.0.get())
            .recording
            .auto_stop;
        AutoStopConfig {
            enabled: options.enabled,
            calendar_end: options.after_calendar_end,
            back_to_back: options.offer_next_meeting,
        }
    }
}

/// Auto-stop's decisions, carried out on the recorder.
pub struct TauriAutoStopActions<R: Runtime> {
    app: AppHandle<R>,
    /// Shows (`true`) or hides the top-right "Ending…" card's window.
    ending_card: fn(&AppHandle<R>, bool),
    /// Whether the card is up, so it is shown and hidden once per notice.
    showing: AtomicBool,
}

impl<R: Runtime> TauriAutoStopActions<R> {
    /// The actions on `app`, with the window that shows the "Ending…" card.
    pub fn new(app: AppHandle<R>, ending_card: fn(&AppHandle<R>, bool)) -> Self {
        let showing = AtomicBool::new(false);
        Self {
            app,
            ending_card,
            showing,
        }
    }
}

impl<R: Runtime> AutoStopActions for TauriAutoStopActions<R> {
    fn changed(&self, state: &AutoStopState) {
        let event = AutoStopChanged {
            state: state.clone(),
        };
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "recording", "could not emit auto-stop-changed: {error}");
        }
        let ending = state.ending.is_some();
        if self.showing.swap(ending, Ordering::Relaxed) != ending {
            (self.ending_card)(&self.app, ending);
        }
    }

    fn stop(&self, reason: StopReason) {
        let Some(recorder) = recorder(&self.app) else {
            return;
        };
        tauri::async_runtime::spawn(async move {
            if let Err(error) = recorder.stop().await {
                log::warn!(target: "recording", "auto-stop ({reason:?}) could not stop: {error}");
            }
        });
    }

    fn split(&self, event: &EventWindow) {
        let Some(recorder) = recorder(&self.app) else {
            return;
        };
        let request = StartRequest {
            title: Some(event.title.clone()),
            link: Some(MeetingLink {
                calendar_event_id: Some(event.id),
                series_id: event.series_id.clone(),
            }),
        };
        tauri::async_runtime::spawn(async move {
            if let Err(error) = recorder.stop().await {
                return log::warn!(target: "recording", "the split could not stop: {error}");
            }
            if let Err(error) = recorder.start(request).await {
                log::warn!(target: "recording", "the split could not start the next note: {error}");
            }
        });
    }
}
