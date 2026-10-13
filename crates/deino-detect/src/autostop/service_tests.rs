//! The auto-stop service over fakes and a fake clock: which app a recording
//! follows, the notice and stop it acts on, the settings, the split offer.

use std::sync::Mutex;

use chrono::{TimeDelta, TimeZone};
use deino_domain::{FakeClock, Millis};

use super::*;
use crate::autostop::{AutoStopConfig, StopReason};

#[derive(Default)]
struct Fakes {
    recording: Mutex<Option<RecordingView>>,
    holders: Mutex<Vec<BundleId>>,
    meeting_app: Mutex<Option<BundleId>>,
    events: Mutex<Vec<EventWindow>>,
    config: Mutex<AutoStopConfig>,
    changes: Mutex<Vec<AutoStopState>>,
    stops: Mutex<Vec<StopReason>>,
    splits: Mutex<Vec<CalendarEventId>>,
}

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap()
}

impl RecordingProbe for Fakes {
    fn recording(&self) -> Option<RecordingView> {
        locked(&self.recording).clone()
    }
}

impl MicrophoneHolders for Fakes {
    fn holds(&self, app: &BundleId) -> bool {
        locked(&self.holders).contains(app)
    }

    fn meeting_app(&self) -> Option<BundleId> {
        locked(&self.meeting_app).clone()
    }
}

impl CalendarWindows for Fakes {
    fn events_near(&self, _now: DateTime<Utc>) -> Vec<EventWindow> {
        locked(&self.events).clone()
    }
}

impl AutoStopSettings for Fakes {
    fn config(&self) -> AutoStopConfig {
        *locked(&self.config)
    }
}

impl AutoStopActions for Fakes {
    fn changed(&self, state: &AutoStopState) {
        locked(&self.changes).push(state.clone());
    }

    fn stop(&self, reason: StopReason) {
        locked(&self.stops).push(reason);
        *locked(&self.recording) = None;
    }

    fn split(&self, event: &EventWindow) {
        locked(&self.splits).push(event.id);
    }
}

struct Rig {
    service: AutoStopService,
    fakes: Arc<Fakes>,
    clock: Arc<FakeClock>,
}

fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap()
}

fn zoom() -> BundleId {
    BundleId::try_from("us.zoom.xos").unwrap()
}

fn rig() -> Rig {
    let fakes = Arc::new(Fakes::default());
    let clock = Arc::new(FakeClock::new(start()));
    let ports = AutoStopPorts {
        recording: fakes.clone(),
        microphone: fakes.clone(),
        calendar: fakes.clone(),
        settings: fakes.clone(),
        actions: fakes.clone(),
    };
    Rig {
        service: AutoStopService::new(ports, clock.clone()),
        fakes,
        clock,
    }
}

impl Rig {
    /// A recording of `meeting` starts now.
    fn record(&self, meeting: &str) {
        *locked(&self.fakes.recording) = Some(RecordingView {
            meeting: MeetingId::new(meeting),
            started_at: self.clock.now(),
            paused: false,
            speech: SpeechTotals::default(),
            disk_full: false,
            event: None,
        });
    }

    fn hold(&self, holds: bool) {
        *locked(&self.fakes.holders) = if holds { vec![zoom()] } else { Vec::new() };
    }

    /// `seconds` ticks a second apart; the call talks while `talking`.
    fn run(&self, seconds: u64, talking: bool) {
        for _ in 0..seconds {
            self.clock.advance(TimeDelta::seconds(1));
            if talking && let Some(view) = locked(&self.fakes.recording).as_mut() {
                view.speech.system = Millis(view.speech.system.0 + 1_000);
            }
            self.service.tick();
        }
    }

    fn stops(&self) -> Vec<StopReason> {
        locked(&self.fakes.stops).clone()
    }

    fn last_change(&self) -> Option<AutoStopState> {
        locked(&self.fakes.changes).last().cloned()
    }
}

#[test]
fn a_detected_meeting_stops_after_its_app_lets_go_of_the_microphone() {
    let rig = rig();
    rig.service.arm(zoom());
    rig.record("meeting-a");
    rig.hold(true);
    rig.run(40, true);

    rig.hold(false);
    rig.run(21, false);
    let notice = rig.last_change().unwrap().ending.unwrap();
    rig.run(15, false);

    assert_eq!(notice.reason, StopReason::MicReleased);
    assert_eq!(rig.stops(), [StopReason::MicReleased]);
    assert_eq!(rig.last_change(), Some(AutoStopState::default()));
}

#[test]
fn an_ad_hoc_recording_follows_the_meeting_app_detection_sees() {
    let rig = rig();
    *locked(&rig.fakes.meeting_app) = Some(zoom());
    rig.record("meeting-a");
    rig.hold(true);
    rig.run(40, true);

    rig.hold(false);
    rig.run(36, false);

    assert_eq!(rig.stops(), [StopReason::MicReleased]);
}

#[test]
fn a_recording_started_before_the_call_finds_its_app_later() {
    let rig = rig();
    rig.record("meeting-a");
    rig.run(10, false);

    *locked(&rig.fakes.meeting_app) = Some(zoom());
    rig.hold(true);
    rig.run(40, true);
    rig.hold(false);
    rig.run(36, false);

    assert_eq!(rig.stops(), [StopReason::MicReleased]);
}

#[test]
fn keep_recording_withdraws_the_notice() {
    let rig = rig();
    rig.service.arm(zoom());
    rig.record("meeting-a");
    rig.hold(true);
    rig.run(40, true);
    rig.hold(false);
    rig.run(21, false);

    rig.service.keep_recording();
    rig.run(60, false);

    assert_eq!(rig.service.state(), AutoStopState::default());
    assert!(rig.stops().is_empty());
}

#[test]
fn switched_off_it_only_stops_for_a_full_disk() {
    let rig = rig();
    locked(&rig.fakes.config).enabled = false;
    rig.service.arm(zoom());
    rig.record("meeting-a");
    rig.hold(true);
    rig.run(40, true);
    rig.hold(false);
    rig.run(20 * 60, false);
    rig.service.will_sleep();
    assert!(rig.stops().is_empty());

    if let Some(view) = locked(&rig.fakes.recording).as_mut() {
        view.disk_full = true;
    }
    rig.run(1, false);

    assert_eq!(rig.stops(), [StopReason::DiskFull]);
}

#[test]
fn sleep_stops_the_recording() {
    let rig = rig();
    rig.record("meeting-a");
    rig.run(5, true);

    rig.service.will_sleep();

    assert_eq!(rig.stops(), [StopReason::Sleep]);
}

fn event(last: u8, start: i64, attendee: &str) -> EventWindow {
    EventWindow {
        id: format!("01926b2c-0000-7000-8000-0000000000{last:02}")
            .parse()
            .unwrap(),
        title: format!("Event {last}"),
        starts_at: start_plus(start),
        ends_at: start_plus(start + 30),
        series_id: None,
        attendees: vec![attendee.to_owned()],
    }
}

fn start_plus(minutes: i64) -> DateTime<Utc> {
    start() + TimeDelta::minutes(minutes)
}

#[test]
fn the_next_meeting_is_offered_once_and_accepting_splits() {
    let rig = rig();
    let design = event(1, 0, "ana@example.com");
    let sales = event(2, 30, "bo@example.com");
    *locked(&rig.fakes.events) = vec![design, sales.clone()];
    rig.record("meeting-a");

    rig.run(30 * 60 + 5, true);
    let offer = rig.service.state().split.unwrap();
    rig.service.accept_split();
    rig.run(60, true);

    assert_eq!(offer.event_id, sales.id);
    assert_eq!(locked(&rig.fakes.splits).clone(), [sales.id]);
    assert_eq!(rig.service.state().split, None, "not offered again");
}

#[test]
fn a_dismissed_offer_is_not_made_again() {
    let rig = rig();
    *locked(&rig.fakes.events) = vec![event(1, 0, "a@x.com"), event(2, 30, "b@x.com")];
    rig.record("meeting-a");
    rig.run(30 * 60 + 5, true);

    rig.service.dismiss_split();
    rig.run(120, true);

    assert_eq!(rig.service.state().split, None);
    assert!(locked(&rig.fakes.splits).is_empty());
}

#[test]
fn the_calendar_rule_applies_only_when_switched_on() {
    let rig = rig();
    *locked(&rig.fakes.events) = vec![event(1, 0, "a@x.com")];
    rig.record("meeting-a");
    rig.run(42 * 60, true);
    rig.run(10 * 60, false);
    assert!(
        rig.stops().is_empty(),
        "the calendar rule is off by default"
    );

    let rig = rig_with_calendar_rule();
    rig.run(42 * 60, true);
    rig.run(3 * 60, false);
    assert_eq!(rig.stops(), [StopReason::CalendarEnded]);
}

/// A recording of the 09:00 event with the calendar rule on.
fn rig_with_calendar_rule() -> Rig {
    let rig = rig();
    locked(&rig.fakes.config).calendar_end = true;
    *locked(&rig.fakes.events) = vec![event(1, 0, "a@x.com")];
    rig.record("meeting-a");
    rig
}
