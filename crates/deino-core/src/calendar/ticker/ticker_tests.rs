//! The ticker over a synced calendar: it shows the reminder a minute
//! ahead, and records an event whose note is open at its start when the
//! rules say so and nothing else records.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_domain::{FakeClock, MeetingId};

use super::{MeetingTicker, OpenNote, RecorderBusy, ReminderWindow, TickerDeps};
use crate::calendar::join::{CallOpener, EventRecorder, JoinService, PortError};
use crate::calendar::rules::{AutoRecordDeps, AutoRecordService};
use crate::calendar::test_calendar::{SyncedCalendar, synced_calendar};
use crate::calendar::test_events::{clock, hours, meeting};
use crate::calendar::{CalendarEvent, SourceEvent};
use crate::settings::SettingsService;

/// What the window was asked to do, in order.
#[derive(Default)]
struct Window(Mutex<Vec<String>>);

impl ReminderWindow for Window {
    fn show(&self, event: &CalendarEvent) {
        self.0.lock().unwrap().push(format!("show {}", event.title));
    }

    fn close(&self) {
        self.0.lock().unwrap().push("close".into());
    }
}

#[derive(Default)]
struct Busy(AtomicBool);

impl RecorderBusy for Busy {
    fn is_recording(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Records which events it started.
#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

#[async_trait]
impl EventRecorder for Recorder {
    async fn record(&self, event: &CalendarEvent) -> Result<MeetingId, PortError> {
        self.0.lock().unwrap().push(event.title.clone());
        Ok(MeetingId::new("meeting-1"))
    }
}

struct NoBrowser;

#[async_trait]
impl CallOpener for NoBrowser {
    async fn open_call(&self, _url: &str) -> Result<(), PortError> {
        Err("Record only never opens the call".into())
    }
}

struct Harness {
    ticker: MeetingTicker,
    clock: Arc<FakeClock>,
    window: Arc<Window>,
    busy: Arc<Busy>,
    recorder: Arc<Recorder>,
    open_note: Arc<OpenNote>,
    settings: Arc<SettingsService>,
    standup: CalendarEvent,
}

/// The auto-record rules over `synced`.
fn rules(synced: &SyncedCalendar, clock: &Arc<FakeClock>) -> Arc<AutoRecordService> {
    Arc::new(AutoRecordService::new(AutoRecordDeps {
        calendar: synced.service.clone(),
        events: synced.repo.clone(),
        series: synced.repo.clone(),
        settings: synced.settings.clone(),
        clock: clock.clone(),
    }))
}

/// "Standup" (external, organised by a colleague, accepted), 30 s after
/// the clock's now.
fn standup() -> SourceEvent {
    let mut standup = meeting("standup", 0);
    standup.title = "Standup".into();
    standup.start = hours(0) + TimeDelta::seconds(30);
    standup.end = standup.start + TimeDelta::minutes(30);
    standup
}

/// The ticker over a calendar holding [`standup`].
async fn harness() -> Harness {
    let synced = synced_calendar(vec![standup()]).await;
    let clock = Arc::new(clock());
    let rules = rules(&synced, &clock);
    let SyncedCalendar {
        service, settings, ..
    } = synced;
    let recorder = Arc::new(Recorder::default());
    let joins = Arc::new(JoinService::new(
        service.clone(),
        Arc::new(NoBrowser),
        recorder.clone(),
    ));
    let standup = service.events_near(hours(0)).remove(0);
    let window = Arc::new(Window::default());
    let busy = Arc::new(Busy::default());
    let open_note = Arc::new(OpenNote::default());
    let ticker = MeetingTicker::new(TickerDeps {
        calendar: service,
        rules,
        joins,
        settings: settings.clone(),
        window: window.clone(),
        recorder: busy.clone(),
        open_note: open_note.clone(),
        clock: clock.clone(),
    });
    Harness {
        ticker,
        clock,
        window,
        busy,
        recorder,
        open_note,
        settings,
        standup,
    }
}

impl Harness {
    async fn record_external(&self) {
        self.settings
            .update(|settings| settings.calendar.auto_record.external = true)
            .await
            .unwrap();
    }

    fn recorded(&self) -> Vec<String> {
        self.recorder.0.lock().unwrap().clone()
    }
}

#[tokio::test]
async fn the_reminder_shows_a_minute_ahead_and_closes_a_minute_later() {
    let h = harness().await;
    h.ticker.tick().await;
    h.clock.advance(TimeDelta::seconds(15));
    h.ticker.tick().await;
    h.clock.advance(TimeDelta::seconds(45));
    h.ticker.tick().await;
    assert_eq!(*h.window.0.lock().unwrap(), ["show Standup", "close"]);
}

#[tokio::test]
async fn do_not_disturb_skips_the_reminder() {
    let h = harness().await;
    h.settings
        .update(|settings| settings.notifications.manual_dnd_mode = true)
        .await
        .unwrap();
    h.ticker.tick().await;
    assert!(h.window.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_open_note_records_its_meeting_at_the_start() {
    let h = harness().await;
    h.record_external().await;
    h.open_note.set(Some(h.standup.id));
    h.ticker.tick().await;
    assert!(h.recorded().is_empty(), "not before the start");
    h.clock.advance(TimeDelta::seconds(30));
    h.ticker.tick().await;
    assert_eq!(h.recorded(), ["Standup"]);
}

#[tokio::test]
async fn the_start_time_needs_the_rules_the_note_and_an_idle_recorder() {
    let h = harness().await;
    h.open_note.set(Some(h.standup.id));
    h.clock.advance(TimeDelta::seconds(30));
    h.ticker.tick().await;
    assert!(h.recorded().is_empty(), "auto-record is off");

    let h = harness().await;
    h.record_external().await;
    h.clock.advance(TimeDelta::seconds(30));
    h.ticker.tick().await;
    assert!(h.recorded().is_empty(), "the note is not open");

    let h = harness().await;
    h.record_external().await;
    h.open_note.set(Some(h.standup.id));
    h.busy.0.store(true, Ordering::SeqCst);
    h.clock.advance(TimeDelta::seconds(30));
    h.ticker.tick().await;
    assert!(h.recorded().is_empty(), "something is recording");
}
