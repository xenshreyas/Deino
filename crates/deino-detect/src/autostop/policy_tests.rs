//! Every stop path of [`AutoStopPolicy`] on a simulated clock: ticks one
//! second apart, never sleeping.

use chrono::TimeZone;

use super::*;

const SECOND: TimeDelta = TimeDelta::seconds(1);

/// A recording ticked second by second.
struct Rig {
    policy: AutoStopPolicy,
    now: DateTime<Utc>,
    seen: Observation,
}

impl Rig {
    /// A recording of a call whose app holds the microphone.
    fn new() -> Self {
        Self {
            policy: AutoStopPolicy::default(),
            now: Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap(),
            seen: Observation {
                meeting: MeetingId::new("meeting-a"),
                paused: false,
                heard: Heard::default(),
                app_holds_mic: Some(true),
                calendar_end: None,
                disk_full: false,
            },
        }
    }

    /// One tick, a second after the last.
    fn tick(&mut self) -> Decision {
        self.now += SECOND;
        self.policy.tick(Some(&self.seen), self.now)
    }

    /// `seconds` ticks; returns every decision that was not `None`, with
    /// the second it came at.
    fn run(&mut self, seconds: i64) -> Vec<(i64, Decision)> {
        (1..=seconds)
            .filter_map(|second| {
                let decision = self.tick();
                (decision != Decision::None).then_some((second, decision))
            })
            .collect()
    }

    /// Ticks for `seconds` with the call talking, then lets the app go.
    fn call_then_release(&mut self, seconds: i64) {
        self.seen.heard.system = true;
        self.run(seconds);
        self.seen.heard.system = false;
        self.seen.app_holds_mic = Some(false);
    }
}

fn ending(reason: StopReason) -> impl Fn(&Decision) -> bool {
    move |decision| matches!(decision, Decision::Ending(notice) if notice.reason == reason)
}

#[test]
fn a_released_microphone_ends_the_recording_after_the_notice() {
    let mut rig = Rig::new();
    rig.call_then_release(60);

    let decisions = rig.run(40);

    assert_eq!(decisions.len(), 2, "{decisions:?}");
    // Released at the first tick; 20 s later the notice, 15 s after it the stop.
    assert_eq!(decisions[0].0, 21, "the release must hold for 20 s");
    assert!(ending(StopReason::MicReleased)(&decisions[0].1));
    assert_eq!(decisions[1], (36, Decision::Stop(StopReason::MicReleased)));
}

#[test]
fn a_short_hold_never_arms_the_release_rule() {
    let mut rig = Rig::new();
    rig.call_then_release(29);

    assert!(rig.run(120).is_empty(), "a 29 s hold is not a call");
}

#[test]
fn speech_on_the_call_restarts_the_release_wait() {
    let mut rig = Rig::new();
    rig.call_then_release(60);
    rig.run(15);
    rig.seen.heard.system = true;
    rig.tick();
    rig.seen.heard.system = false;

    let decisions = rig.run(25);

    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].0, 20, "20 s after the call last spoke");
}

#[test]
fn the_app_taking_the_microphone_back_withdraws_the_notice() {
    let mut rig = Rig::new();
    rig.call_then_release(60);
    rig.run(22);

    rig.seen.app_holds_mic = Some(true);

    assert_eq!(rig.tick(), Decision::Withdraw);
    assert!(rig.run(120).is_empty());
}

#[test]
fn keep_recording_ignores_the_release_until_the_app_holds_the_mic_again() {
    let mut rig = Rig::new();
    rig.call_then_release(60);
    rig.run(21);
    assert_eq!(rig.policy.keep_recording(rig.now), Decision::Withdraw);
    assert!(rig.run(300).is_empty(), "kept");

    rig.seen.app_holds_mic = Some(true);
    rig.run(5);
    rig.seen.app_holds_mic = Some(false);
    let decisions = rig.run(21);

    assert_eq!(decisions.len(), 1, "the next release counts again");
    assert!(ending(StopReason::MicReleased)(&decisions[0].1));
}

#[test]
fn fifteen_minutes_without_speech_end_the_recording() {
    let mut rig = Rig::new();
    rig.seen.app_holds_mic = None;

    let decisions = rig.run(15 * 60 + 16);

    // First seen at the first tick: 15 minutes later the notice, then the stop.
    assert_eq!(decisions.len(), 2, "{decisions:?}");
    assert_eq!(decisions[0].0, 15 * 60 + 1);
    assert!(ending(StopReason::NoSpeech)(&decisions[0].1));
    assert_eq!(
        decisions[1],
        (15 * 60 + 16, Decision::Stop(StopReason::NoSpeech))
    );
}

#[test]
fn speech_during_the_notice_keeps_the_recording_going() {
    let mut rig = Rig::new();
    rig.seen.app_holds_mic = None;
    rig.run(15 * 60 + 5);

    rig.seen.heard.mic = true;

    assert_eq!(rig.tick(), Decision::Withdraw);
    rig.seen.heard.mic = false;
    assert!(rig.run(14 * 60).is_empty());
}

#[test]
fn keep_recording_after_silence_restarts_the_silence_window() {
    let mut rig = Rig::new();
    rig.seen.app_holds_mic = None;
    rig.run(15 * 60 + 1);

    assert_eq!(rig.policy.keep_recording(rig.now), Decision::Withdraw);
    let decisions = rig.run(15 * 60);

    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].0, 15 * 60);
}

#[test]
fn the_calendar_rule_waits_for_ten_minutes_past_the_end_and_two_quiet_minutes() {
    let mut rig = Rig::new();
    rig.seen.app_holds_mic = None;
    rig.seen.calendar_end = Some(rig.now + TimeDelta::minutes(5));
    rig.seen.heard.mic = true;
    assert!(
        rig.run(15 * 60).is_empty(),
        "still talking 10 min after the end"
    );

    rig.seen.heard.mic = false;
    let decisions = rig.run(3 * 60);

    assert_eq!(decisions[0].0, 2 * 60);
    assert!(ending(StopReason::CalendarEnded)(&decisions[0].1));
    assert_eq!(decisions[1].1, Decision::Stop(StopReason::CalendarEnded));
}

#[test]
fn without_a_calendar_end_the_calendar_rule_never_applies() {
    let mut rig = Rig::new();
    rig.seen.app_holds_mic = None;
    rig.seen.heard.system = true;
    rig.run(60 * 60);
    rig.seen.heard.system = false;

    let decisions = rig.run(5 * 60);

    assert!(decisions.is_empty(), "{decisions:?}");
}

#[test]
fn sleep_stops_at_once_and_only_once() {
    let mut rig = Rig::new();
    rig.run(10);

    assert_eq!(rig.policy.will_sleep(), Decision::Stop(StopReason::Sleep));
    assert_eq!(rig.policy.will_sleep(), Decision::None);
    assert!(rig.run(60).is_empty(), "the recording is already ending");
}

#[test]
fn a_long_tick_gap_counts_as_sleep() {
    let mut rig = Rig::new();
    rig.run(10);

    rig.now += TimeDelta::minutes(30);

    assert_eq!(rig.tick(), Decision::Stop(StopReason::Sleep));
}

#[test]
fn a_short_stall_restarts_the_windows_instead_of_counting_as_silence() {
    let mut rig = Rig::new();
    rig.seen.app_holds_mic = None;
    rig.run(14 * 60 + 50);

    rig.now += TimeDelta::seconds(30);
    let decisions = rig.run(60);

    assert!(decisions.is_empty(), "{decisions:?}");
}

#[test]
fn a_full_disk_stops_at_once() {
    let mut rig = Rig::new();
    rig.run(5);
    rig.seen.disk_full = true;

    assert_eq!(rig.tick(), Decision::Stop(StopReason::DiskFull));
}

#[test]
fn nothing_but_sleep_ends_a_paused_recording() {
    let mut rig = Rig::new();
    rig.call_then_release(60);
    rig.run(25);
    rig.seen.paused = true;

    assert_eq!(rig.tick(), Decision::Withdraw);
    assert!(rig.run(60 * 60).is_empty());
    assert_eq!(rig.policy.will_sleep(), Decision::Stop(StopReason::Sleep));
}

#[test]
fn a_new_meeting_starts_with_a_clean_slate() {
    let mut rig = Rig::new();
    rig.call_then_release(60);
    rig.run(25);

    rig.seen.meeting = MeetingId::new("meeting-b");

    assert!(
        rig.run(60).is_empty(),
        "the new meeting's app never held the mic"
    );
    assert_eq!(rig.policy.tick(None, rig.now + SECOND), Decision::None);
}
