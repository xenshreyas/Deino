//! Join & record: capture starts within 2 s even when the browser is slow,
//! each mode does only what it says, and failures are reported.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_domain::{CalendarEventId, MeetingId};
use tokio::time::Instant;

use super::{CallOpener, EventLookup, EventRecorder, JoinError, JoinMode, JoinService, PortError};
use crate::calendar::test_events::{hours, stored};
use crate::calendar::{CalendarError, CalendarEvent};

/// The card's bound: Join & record starts capture within 2 s.
const CAPTURE_BOUND: Duration = Duration::from_secs(2);
/// A browser that takes its time to come up.
const SLOW_BROWSER: Duration = Duration::from_secs(5);
/// The fake recorder's start-up, like the live recorder's first audio.
const RECORDER_START: Duration = Duration::from_millis(300);

/// Opens after [`SLOW_BROWSER`], or fails.
#[derive(Default)]
struct FakeOpener {
    opened: Mutex<Vec<String>>,
    fails: bool,
}

#[async_trait]
impl CallOpener for FakeOpener {
    async fn open_call(&self, url: &str) -> Result<(), PortError> {
        tokio::time::sleep(SLOW_BROWSER).await;
        if self.fails {
            return Err("no browser".into());
        }
        self.opened.lock().unwrap().push(url.to_owned());
        Ok(())
    }
}

/// Starts after [`RECORDER_START`] and notes when capture began.
struct FakeRecorder {
    began: Instant,
    captured_after: Mutex<Option<Duration>>,
}

#[async_trait]
impl EventRecorder for FakeRecorder {
    async fn record(&self, event: &CalendarEvent) -> Result<MeetingId, PortError> {
        tokio::time::sleep(RECORDER_START).await;
        *self.captured_after.lock().unwrap() = Some(self.began.elapsed());
        Ok(MeetingId::new(format!("meeting-for-{}", event.title)))
    }
}

/// One stored event, by ID.
struct OneEvent(CalendarEvent);

#[async_trait]
impl EventLookup for OneEvent {
    async fn event(&self, id: CalendarEventId) -> Result<CalendarEvent, CalendarError> {
        if id == self.0.id {
            Ok(self.0.clone())
        } else {
            Err(CalendarError::EventNotFound(id))
        }
    }
}

/// A two-person call with a Meet link, starting in a minute.
fn standup() -> CalendarEvent {
    let mut event = stored("standup", 0);
    event.start_at = hours(0) + TimeDelta::minutes(1);
    event.conference_url = Some("https://meet.google.com/abc-defg-hij".into());
    event
}

struct Harness {
    service: JoinService,
    opener: Arc<FakeOpener>,
    recorder: Arc<FakeRecorder>,
    event: CalendarEventId,
}

fn harness(opener: FakeOpener, event: CalendarEvent) -> Harness {
    let id = event.id;
    let opener = Arc::new(opener);
    let recorder = Arc::new(FakeRecorder {
        began: Instant::now(),
        captured_after: Mutex::new(None),
    });
    let service = JoinService::new(Arc::new(OneEvent(event)), opener.clone(), recorder.clone());
    Harness {
        service,
        opener,
        recorder,
        event: id,
    }
}

fn captured_after(recorder: &FakeRecorder) -> Option<Duration> {
    *recorder.captured_after.lock().unwrap()
}

#[tokio::test(start_paused = true)]
async fn join_and_record_captures_within_two_seconds_of_a_slow_browser() {
    let Harness {
        service,
        opener,
        recorder,
        event,
    } = harness(FakeOpener::default(), standup());

    let meeting = service.join(event, JoinMode::JoinAndRecord).await.unwrap();

    assert_eq!(meeting, Some(MeetingId::new("meeting-for-standup")));
    let after = captured_after(&recorder).unwrap();
    assert!(after <= CAPTURE_BOUND, "capture began after {after:?}");
    assert_eq!(opener.opened.lock().unwrap().len(), 1, "the call opened too");
}

#[tokio::test(start_paused = true)]
async fn each_mode_does_only_what_it_says() {
    let Harness {
        service,
        opener,
        recorder,
        event,
    } = harness(FakeOpener::default(), standup());

    assert_eq!(service.join(event, JoinMode::JoinOnly).await.unwrap(), None);
    assert!(captured_after(&recorder).is_none());

    let before = opener.opened.lock().unwrap().len();
    assert!(service.join(event, JoinMode::RecordOnly).await.unwrap().is_some());
    assert_eq!(opener.opened.lock().unwrap().len(), before, "record only opens nothing");
}

#[tokio::test(start_paused = true)]
async fn a_browser_failure_keeps_the_recording_but_fails_join_only() {
    let failing = FakeOpener {
        fails: true,
        ..FakeOpener::default()
    };
    let Harness { service, event, .. } = harness(failing, standup());

    assert!(service.join(event, JoinMode::JoinAndRecord).await.unwrap().is_some());
    let only = service.join(event, JoinMode::JoinOnly).await;
    assert!(matches!(only, Err(JoinError::Open(_))));
}

#[tokio::test(start_paused = true)]
async fn joining_an_event_without_a_link_is_refused() {
    let mut in_person = standup();
    in_person.conference_url = None;
    let Harness { service, event, .. } = harness(FakeOpener::default(), in_person);

    let refused = service.join(event, JoinMode::JoinOnly).await;
    assert!(matches!(refused, Err(JoinError::NoCallLink)));
    assert!(service.join(event, JoinMode::JoinAndRecord).await.unwrap().is_some());
}
