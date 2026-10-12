use chrono::TimeDelta;
use deino_domain::FakeClock;

use super::*;
use crate::fake::{FakeRecorder, FakeSettings, RecordedSink};
use crate::model::{MeetingKind, Platform};
use crate::policy::{PROMPT_TIMEOUT, SHORT_DWELL};
use crate::testing::{SLACK, ZOOM, app, client, id, start};

/// A service over fakes, with handles to drive and inspect them.
struct Harness {
    clock: Arc<FakeClock>,
    sink: RecordedSink,
    recorder: FakeRecorder,
    settings: FakeSettings,
    service: DetectionService,
}

impl Harness {
    fn new(auto_start: Option<TimeDelta>) -> Self {
        let clock = Arc::new(FakeClock::new(start()));
        let (sink, recorder) = (RecordedSink::default(), FakeRecorder::default());
        let settings = FakeSettings::default();
        settings.0.set(DetectionConfig {
            enabled: true,
            auto_start,
            ..DetectionConfig::default()
        });
        let ports = DetectionPorts {
            sink: Arc::new(sink.clone()),
            recorder: Arc::new(recorder.clone()),
            status: Arc::new(recorder.clone()),
            config: Arc::new(settings.clone()),
            ignore_list: Arc::new(settings.clone()),
        };
        let service = DetectionService::new(ports, clock.clone());
        Self {
            clock,
            sink,
            recorder,
            settings,
            service,
        }
    }

    fn after(&self, delta: TimeDelta, snapshot: &SignalSnapshot) {
        self.clock.advance(delta);
        self.service.evaluate(snapshot);
    }

    /// Holds `snapshot` long enough to prompt, and returns the prompt.
    fn detect(&self, snapshot: &SignalSnapshot) -> Detection {
        self.after(TimeDelta::zero(), snapshot);
        self.after(SHORT_DWELL, snapshot);
        self.service.current().expect("a prompt")
    }

    fn events(&self) -> Vec<DetectionEvent> {
        self.sink.0.get()
    }
}

fn in_call(bundle: &str) -> SignalSnapshot {
    SignalSnapshot {
        clients: vec![client(bundle, true)],
        microphone_running: true,
        ..SignalSnapshot::default()
    }
}

#[test]
fn a_held_call_publishes_one_detection_with_its_title_kind() {
    let harness = Harness::new(None);

    let detection = harness.detect(&in_call(SLACK));

    assert_eq!(
        (detection.kind, detection.platform),
        (MeetingKind::Huddle, Platform::Slack)
    );
    assert_eq!(detection.app, app(SLACK));
    assert_eq!(
        (detection.detected_at, detection.auto_start_at),
        (start() + SHORT_DWELL, None)
    );
    assert_eq!(
        harness.events(),
        vec![DetectionEvent::MeetingDetected(detection)]
    );
}

#[test]
fn take_notes_closes_the_prompt_and_starts_recording_with_the_notes_open() {
    let harness = Harness::new(None);
    let detection = harness.detect(&in_call(ZOOM));

    harness.service.take_notes(detection.id).unwrap();

    assert_eq!(
        harness.events().last(),
        Some(&DetectionEvent::PromptClosed(detection.id))
    );
    assert_eq!(
        harness.recorder.requests.get(),
        vec![RecordingRequest {
            detection: detection.clone(),
            open_notes: true
        }]
    );
    assert_eq!(harness.service.current(), None);
    assert!(matches!(
        harness.service.take_notes(detection.id),
        Err(DetectError::PromptClosed)
    ));
}

#[test]
fn record_now_closes_the_prompt_and_records_without_opening_the_notes() {
    let harness = Harness::new(Some(TimeDelta::seconds(5)));
    let detection = harness.detect(&in_call(ZOOM));

    harness.service.record_now(detection.id).unwrap();

    assert_eq!(
        harness.events().last(),
        Some(&DetectionEvent::PromptClosed(detection.id))
    );
    assert_eq!(
        harness.recorder.requests.get(),
        vec![RecordingRequest {
            detection: detection.clone(),
            open_notes: false
        }]
    );
    assert!(matches!(
        harness.service.record_now(detection.id),
        Err(DetectError::PromptClosed)
    ));
}

#[test]
fn dismiss_closes_the_prompt_without_recording() {
    let harness = Harness::new(None);
    let detection = harness.detect(&in_call(ZOOM));

    harness.service.dismiss(detection.id).unwrap();

    assert_eq!(
        harness.events().last(),
        Some(&DetectionEvent::PromptClosed(detection.id))
    );
    assert!(harness.recorder.requests.get().is_empty());
}

#[tokio::test]
async fn always_ignore_saves_the_app_and_it_never_prompts_again() {
    let harness = Harness::new(None);
    let detection = harness.detect(&in_call(ZOOM));

    harness.service.ignore_app(detection.id).await.unwrap();
    harness.after(TimeDelta::minutes(1), &SignalSnapshot::default());
    harness.after(TimeDelta::minutes(1), &SignalSnapshot::default());
    harness.after(TimeDelta::minutes(1), &in_call(ZOOM));
    harness.after(TimeDelta::minutes(1), &in_call(ZOOM));

    assert!(harness.settings.0.get().ignored.contains(&id(ZOOM)));
    assert_eq!(harness.service.current(), None);
}

#[test]
fn auto_start_records_when_the_countdown_runs_out_without_taking_focus() {
    let harness = Harness::new(Some(TimeDelta::seconds(10)));
    let detection = harness.detect(&in_call(ZOOM));
    let countdown_ends = start() + SHORT_DWELL + TimeDelta::seconds(10);

    assert_eq!(detection.auto_start_at, Some(countdown_ends));
    assert_eq!(harness.service.next_deadline(), Some(countdown_ends));
    harness.after(TimeDelta::seconds(9), &in_call(ZOOM));
    assert!(harness.recorder.requests.get().is_empty());

    harness.after(TimeDelta::seconds(1), &in_call(ZOOM));

    assert_eq!(
        harness.recorder.requests.get(),
        vec![RecordingRequest {
            detection: detection.clone(),
            open_notes: false
        }]
    );
    assert_eq!(
        harness.events().last(),
        Some(&DetectionEvent::PromptClosed(detection.id))
    );
}

#[test]
fn dont_record_during_the_countdown_cancels_the_auto_start() {
    let harness = Harness::new(Some(TimeDelta::seconds(5)));
    let detection = harness.detect(&in_call(ZOOM));

    harness.service.dismiss(detection.id).unwrap();
    harness.after(TimeDelta::seconds(10), &in_call(ZOOM));

    assert!(harness.recorder.requests.get().is_empty());
}

#[test]
fn the_meeting_ending_and_the_prompt_timing_out_are_published() {
    let harness = Harness::new(None);
    let first = harness.detect(&in_call(ZOOM));
    harness.after(PROMPT_TIMEOUT, &in_call(ZOOM));
    harness.after(TimeDelta::seconds(5), &SignalSnapshot::default());
    harness.after(TimeDelta::seconds(5), &SignalSnapshot::default());
    let second = harness.detect(&in_call(ZOOM));
    harness.after(TimeDelta::seconds(5), &SignalSnapshot::default());
    harness.after(TimeDelta::seconds(5), &SignalSnapshot::default());

    let events = harness.events();
    assert!(events.contains(&DetectionEvent::PromptClosed(first.id)));
    assert_eq!(
        events.last(),
        Some(&DetectionEvent::MeetingEnded(second.id))
    );
}

#[test]
fn nothing_prompts_while_recording_or_switched_off() {
    let harness = Harness::new(None);
    harness.recorder.recording.set(true);
    harness.after(TimeDelta::zero(), &in_call(ZOOM));
    harness.after(TimeDelta::seconds(10), &in_call(ZOOM));
    harness.recorder.recording.set(false);
    harness.settings.0.lock().enabled = false;
    harness.after(TimeDelta::seconds(10), &in_call(SLACK));
    harness.after(TimeDelta::seconds(10), &in_call(SLACK));

    assert!(harness.events().is_empty());
}
