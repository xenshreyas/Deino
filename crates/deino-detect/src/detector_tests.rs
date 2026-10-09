//! The detector thread end to end, over the fake Mac and the real clock:
//! these wait for events on a channel, never by sleeping.

use std::sync::mpsc::{self, Sender};

use deino_domain::SystemClock;

use super::*;
use crate::calendar::NoCalendar;
use crate::event::DetectionEvent;
use crate::fake::{FakeMac, FakeRecorder, FakeSettings};
use crate::model::Pid;
use crate::outputs::{DetectionConfig, DetectionSink};
use crate::service::DetectionPorts;
use crate::testing::{ZOOM, app, process};

/// Longer than any dwell, grace or poll the detector waits through.
const EVENT_TIMEOUT: Duration = Duration::from_secs(8);

/// Forwards every event to the test.
struct ChannelSink(Sender<DetectionEvent>);

impl DetectionSink for ChannelSink {
    fn publish(&self, event: DetectionEvent) {
        // The test may have finished listening; the event is then moot.
        self.0.send(event).unwrap_or(());
    }
}

/// Everything the detector thread needs besides sensors, publishing to `sink`.
fn parts(sink: Arc<dyn DetectionSink>) -> DetectorParts {
    let settings = FakeSettings::default();
    settings.0.set(DetectionConfig {
        enabled: true,
        ..DetectionConfig::default()
    });
    let recorder = FakeRecorder::default();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let service = DetectionService::new(
        DetectionPorts {
            sink,
            recorder: Arc::new(recorder.clone()),
            status: Arc::new(recorder),
            config: Arc::new(settings.clone()),
            ignore_list: Arc::new(settings),
        },
        Arc::clone(&clock),
    );
    DetectorParts {
        service: Arc::new(service),
        calendar: Arc::new(NoCalendar),
        clock,
    }
}

/// A detector over `mac`, and the receiving end of its events.
fn start(mac: &FakeMac) -> (Detector, mpsc::Receiver<DetectionEvent>) {
    let (sender, events) = mpsc::channel();
    let sensors_mac = mac.clone();
    let detector = Detector::spawn(
        move || Ok(sensors_mac.sensors(Pid(1))),
        parts(Arc::new(ChannelSink(sender))),
    )
    .unwrap();
    (detector, events)
}

fn zoom_in_a_call(mac: &FakeMac) {
    mac.apps.pids.lock().insert(Pid(10), app(ZOOM));
    mac.processes
        .reading
        .set(Some(vec![process(10, Some(ZOOM), true, true)]));
}

#[test]
fn a_meeting_app_is_prompted_and_its_end_is_published() {
    let mac = FakeMac::idle();
    zoom_in_a_call(&mac);
    let (detector, events) = start(&mac);

    let detected = events.recv_timeout(EVENT_TIMEOUT).unwrap();
    mac.processes.reading.set(Some(Vec::new()));
    detector.wake();
    let ended = events.recv_timeout(EVENT_TIMEOUT).unwrap();

    let DetectionEvent::MeetingDetected(detection) = detected else {
        panic!("{detected:?}")
    };
    assert_eq!(detection.app, app(ZOOM));
    assert_eq!(ended, DetectionEvent::MeetingEnded(detection.id));
    assert!(
        mac.processes.waker.get().is_some(),
        "the process listener was registered"
    );
    assert!(
        mac.processes.refreshes.get() > 0,
        "watches are refreshed on every tick"
    );
}

#[test]
fn without_listeners_the_detector_polls_and_still_detects() {
    let mut mac = FakeMac::idle();
    mac.processes.unwatchable = true;
    let (_detector, events) = start(&mac);

    // No wake: only the poll can notice the call.
    zoom_in_a_call(&mac);

    assert!(matches!(
        events.recv_timeout(EVENT_TIMEOUT),
        Ok(DetectionEvent::MeetingDetected(_))
    ));
}

#[test]
fn a_platform_without_sensors_stops_quietly() {
    let (sender, events) = mpsc::channel();

    let detector = Detector::spawn(
        || Err(DetectError::Unsupported),
        parts(Arc::new(ChannelSink(sender))),
    )
    .unwrap();
    drop(detector);

    assert!(events.try_recv().is_err());
}
