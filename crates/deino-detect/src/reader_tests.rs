use chrono::{DateTime, Utc};
use deino_domain::FakeClock;

use super::*;
use crate::calendar::{LinkedEvent, NoCalendar};
use crate::fake::FakeMac;
use crate::testing::{CHROME, ZOOM, app, event, process, running, start};

const OWN_PID: Pid = Pid(1);

fn reader() -> SignalReader {
    SignalReader::new(Arc::new(NoCalendar), Arc::new(FakeClock::new(start())))
}

/// A Mac where `apps` own the given pids.
fn mac_with(processes: Vec<AudioProcess>, apps: &[(i32, &str)]) -> FakeMac {
    let mac = FakeMac::idle();
    mac.processes.reading.set(Some(processes));
    for (pid, bundle) in apps {
        mac.apps.pids.lock().insert(Pid(*pid), app(bundle));
    }
    mac
}

#[test]
fn capture_and_playback_helpers_merge_into_one_full_duplex_app() {
    let mac = mac_with(
        vec![
            process(10, Some("com.google.Chrome.helper"), true, false),
            process(11, Some("com.google.Chrome.helper"), false, true),
        ],
        &[(10, CHROME), (11, CHROME)],
    );

    let snapshot = reader().read(&mac.sensors(OWN_PID));

    assert_eq!(
        snapshot.clients,
        vec![MicClient {
            app: app(CHROME),
            pid: Pid(10),
            full_duplex: true
        }]
    );
}

#[test]
fn deino_core_speech_and_idle_processes_are_never_clients() {
    let mac = mac_with(
        vec![
            process(OWN_PID.0, Some("com.xenshreyas.deino"), true, true),
            process(20, Some("com.apple.CoreSpeech"), true, false),
            process(21, Some(ZOOM), false, false),
        ],
        &[(20, "process.corespeechd"), (21, ZOOM)],
    );

    assert!(reader().read(&mac.sensors(OWN_PID)).clients.is_empty());
}

#[test]
fn rooms_are_read_only_while_something_captures_and_accessibility_is_granted() {
    let mac = mac_with(vec![process(10, Some(CHROME), true, true)], &[(10, CHROME)]);
    mac.apps.running.set(vec![running(CHROME, 10, true)]);
    mac.tabs.urls.lock().insert(
        Pid(10),
        vec![
            "https://example.com".to_owned(),
            "https://meet.google.com/abc-defg-hij".to_owned(),
        ],
    );

    let without_access = reader().read(&mac.sensors(OWN_PID));
    mac.tabs.available.set(true);
    let with_access = reader().read(&mac.sensors(OWN_PID));
    mac.processes.reading.set(Some(Vec::new()));
    let idle = reader().read(&mac.sensors(OWN_PID));

    assert!(without_access.browser_meetings.is_empty());
    assert_eq!(with_access.browser_meetings.len(), 1);
    assert!(with_access.browser_meetings[0].focused);
    assert_eq!(
        with_access.browser_meetings[0].url.room,
        "meet.google.com/abc-defg-hij"
    );
    assert!(idle.browser_meetings.is_empty());
}

#[test]
fn the_frontmost_browser_is_read_while_the_microphone_runs() {
    let mac = FakeMac::idle();
    mac.microphone.reading.set(Some(true));
    mac.tabs.available.set(true);
    mac.apps.running.set(vec![running(CHROME, 10, true)]);
    mac.tabs
        .urls
        .lock()
        .insert(Pid(10), vec!["https://us02web.zoom.us/j/123".to_owned()]);

    let snapshot = reader().read(&mac.sensors(OWN_PID));

    assert_eq!(snapshot.frontmost, Some(app(CHROME)));
    assert_eq!(snapshot.browser_meetings.len(), 1);
}

#[test]
fn a_failing_sensor_reads_as_idle() {
    let mac = FakeMac::default();

    let snapshot = reader().read(&mac.sensors(OWN_PID));

    assert_eq!(snapshot, SignalSnapshot::default());
}

/// A calendar with one event.
struct OneEvent(LinkedEvent);

impl CalendarLookup for OneEvent {
    fn linked_event(&self, now: DateTime<Utc>) -> Option<LinkedEvent> {
        crate::calendar::pick_linked([self.0.clone()], now)
    }
}

#[test]
fn the_linked_calendar_event_is_part_of_the_snapshot() {
    let mut reader = SignalReader::new(
        Arc::new(OneEvent(event(start()))),
        Arc::new(FakeClock::new(start())),
    );

    assert_eq!(
        reader.read(&FakeMac::idle().sensors(OWN_PID)).event,
        Some(event(start()))
    );
}
