//! [`AutoStopPolicy`]: when a recording should end on its own (CAP-07). A
//! pure state machine over one [`Observation`] per tick and the time, so
//! every stop path is tested with a fake clock.
//!
//! It is OpenWhispr's `meetingAutoEndController` (MIT; tick-driven and
//! timer-free), reimplemented with CAP-07's rules, anarlog's confirm step and
//! Granola's limits (`audio-capture.md` §7):
//!
//! | Reason | Rule |
//! |---|---|
//! | [`StopReason::MicReleased`] | the meeting app held the microphone for [`MIN_OWNERSHIP`], then released it for [`MIC_RELEASE`] with no speech on the call meanwhile (system speech restarts the wait) |
//! | [`StopReason::NoSpeech`] | nobody spoke on either lane for [`NO_SPEECH`] |
//! | [`StopReason::CalendarEnded`] | (optional) the linked event ended [`CALENDAR_GRACE`] ago and nobody spoke for [`CALENDAR_QUIET`] |
//! | [`StopReason::Sleep`] | the Mac goes to sleep, or a tick comes [`SLEEP_GAP`] late |
//! | [`StopReason::DiskFull`] | recording health reports a full disk |
//!
//! The first three show an "Ending…" notice for [`ENDING_NOTICE`] with
//! "Keep recording" first ([`Decision::Ending`]); it is withdrawn when the
//! reason goes away (the app takes the microphone back, someone speaks).
//! "Keep recording" silences that reason: the released app until it holds
//! the microphone again, the silence window until it has run out again, the
//! calendar rule for the rest of the recording. Sleep and a full disk stop
//! at once. While paused, nothing but sleep and a full disk can stop it.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::MeetingId;
use serde::Serialize;

/// How long the meeting app must have released the microphone (owner, Oct
/// 2026: 20 s plus the notice felt like it never stopped).
pub const MIC_RELEASE: TimeDelta = TimeDelta::seconds(10);

/// How long the app must have held the microphone for its release to count
/// (OpenWhispr: a Siri request or a permission prompt is not a call).
pub const MIN_OWNERSHIP: TimeDelta = TimeDelta::seconds(10);

/// How long nobody may speak before the recording ends (Granola: 15 min).
pub const NO_SPEECH: TimeDelta = TimeDelta::minutes(15);

/// How long after the linked event's end the calendar rule applies.
pub const CALENDAR_GRACE: TimeDelta = TimeDelta::minutes(10);

/// How long it must have been quiet for the calendar rule.
pub const CALENDAR_QUIET: TimeDelta = TimeDelta::minutes(2);

/// How long the "Ending…" notice shows before the recording stops.
pub const ENDING_NOTICE: TimeDelta = TimeDelta::seconds(15);

/// A tick this late means the Mac slept without telling us.
pub const SLEEP_GAP: TimeDelta = TimeDelta::seconds(60);

/// A tick this late means the process stalled: windows restart, so the gap
/// never counts as silence or as a released microphone.
pub const TICK_GAP: TimeDelta = TimeDelta::seconds(5);

/// Why a recording ends on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    /// The meeting app let go of the microphone: the call ended.
    MicReleased,
    /// Nobody has spoken for a long time.
    NoSpeech,
    /// The calendar event is over and it has gone quiet.
    CalendarEnded,
    /// The Mac went to sleep.
    Sleep,
    /// The disk is full.
    DiskFull,
}

/// The "Ending…" notice: the recording stops at `stop_at` unless kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EndingNotice {
    /// Why it is ending.
    pub reason: StopReason,
    /// When it stops.
    pub stop_at: DateTime<Utc>,
}

/// The recording as one tick sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    /// The meeting being recorded.
    pub meeting: MeetingId,
    /// Capture is paused.
    pub paused: bool,
    /// Who spoke since the previous tick.
    pub heard: Heard,
    /// Whether the meeting app still holds the microphone; `None` when the
    /// recording has no meeting app to follow.
    pub app_holds_mic: Option<bool>,
    /// When the linked calendar event ends, if the calendar rule is on.
    pub calendar_end: Option<DateTime<Utc>>,
    /// Recording health reports a full disk.
    pub disk_full: bool,
}

/// Who spoke since the previous tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Heard {
    /// Someone spoke on the microphone.
    pub mic: bool,
    /// Someone spoke on the call (system audio).
    pub system: bool,
}

/// What to do after a tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Nothing changes.
    None,
    /// Show the "Ending…" notice.
    Ending(EndingNotice),
    /// Withdraw the notice: the reason went away.
    Withdraw,
    /// Stop the recording now.
    Stop(StopReason),
}

/// What the policy remembers about the meeting app's hold on the microphone.
#[derive(Clone, Copy, Debug, Default)]
struct Hold {
    since: Option<DateTime<Utc>>,
    owned: bool,
    released_since: Option<DateTime<Utc>>,
    /// "Keep recording" after a release: ignored until it holds again.
    kept: bool,
}

/// One recording's auto-stop state.
#[derive(Clone, Debug)]
struct Watch {
    meeting: MeetingId,
    last_speech: DateTime<Utc>,
    hold: Hold,
    calendar_kept: bool,
    ending: Option<EndingNotice>,
    /// A stop was decided; the recording is ending, nothing more to decide.
    stopping: bool,
}

/// The auto-stop state machine.
#[derive(Debug, Default)]
pub struct AutoStopPolicy {
    watch: Option<Watch>,
    last_tick: Option<DateTime<Utc>>,
}

impl AutoStopPolicy {
    /// One tick at `now`; `None` while nothing is recording.
    pub fn tick(&mut self, observation: Option<&Observation>, now: DateTime<Utc>) -> Decision {
        let gap = self.last_tick.map_or(TimeDelta::zero(), |last| now - last);
        self.last_tick = Some(now);
        let Some(observation) = observation else {
            return self.forget();
        };
        let slept = gap >= SLEEP_GAP && self.watch.is_some();
        let watch = self.follow(observation, now);
        if watch.stopping {
            return Decision::None;
        }
        if slept {
            return watch.stop(StopReason::Sleep);
        }
        if gap >= TICK_GAP {
            watch.restart_windows(now);
        }
        if observation.disk_full {
            return watch.stop(StopReason::DiskFull);
        }
        watch.observe(observation, now);
        watch.decide(observation, now)
    }

    /// The Mac is about to sleep: a running recording stops.
    pub fn will_sleep(&mut self) -> Decision {
        match self.watch.as_mut() {
            Some(watch) if !watch.stopping => watch.stop(StopReason::Sleep),
            _ => Decision::None,
        }
    }

    /// "Keep recording": withdraws the notice and silences its reason.
    pub fn keep_recording(&mut self, now: DateTime<Utc>) -> Decision {
        let Some(watch) = self.watch.as_mut() else {
            return Decision::None;
        };
        let Some(ending) = watch.ending.take() else {
            return Decision::None;
        };
        match ending.reason {
            StopReason::MicReleased => watch.hold.kept = true,
            StopReason::NoSpeech => watch.last_speech = now,
            StopReason::CalendarEnded => watch.calendar_kept = true,
            StopReason::Sleep | StopReason::DiskFull => {}
        }
        Decision::Withdraw
    }

    /// The notice on screen, if any.
    pub fn ending(&self) -> Option<EndingNotice> {
        self.watch.as_ref().and_then(|watch| watch.ending)
    }

    /// Nothing is recording: forget the last recording.
    fn forget(&mut self) -> Decision {
        let showing = self.ending().is_some();
        self.watch = None;
        if showing {
            Decision::Withdraw
        } else {
            Decision::None
        }
    }

    /// The watch for `observation`'s meeting, fresh for a new meeting.
    fn follow(&mut self, observation: &Observation, now: DateTime<Utc>) -> &mut Watch {
        let fresh = self
            .watch
            .as_ref()
            .is_none_or(|watch| watch.meeting != observation.meeting);
        if fresh {
            self.watch = Some(Watch::new(observation.meeting.clone(), now));
        }
        self.watch
            .get_or_insert_with(|| Watch::new(observation.meeting.clone(), now))
    }
}

mod watch;

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
