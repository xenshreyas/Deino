//! One recording's auto-stop state: what it has observed, which stop
//! reason is due, and the notice on screen.

use chrono::{DateTime, Utc};
use deino_domain::MeetingId;

use super::{
    CALENDAR_GRACE, CALENDAR_QUIET, Decision, ENDING_NOTICE, EndingNotice, Hold, MIC_RELEASE,
    MIN_OWNERSHIP, NO_SPEECH, Observation, StopReason, Watch,
};

/// The reasons that end a recording after a notice, in priority order.
const NOTICED: [StopReason; 3] = [
    StopReason::MicReleased,
    StopReason::NoSpeech,
    StopReason::CalendarEnded,
];

impl Watch {
    /// A recording first seen at `now`.
    pub(super) fn new(meeting: MeetingId, now: DateTime<Utc>) -> Self {
        Self {
            meeting,
            last_speech: now,
            hold: Hold::default(),
            calendar_kept: false,
            ending: None,
            stopping: false,
        }
    }

    /// Decides to stop for `reason`: the notice goes, nothing more follows.
    pub(super) fn stop(&mut self, reason: StopReason) -> Decision {
        self.ending = None;
        self.stopping = true;
        Decision::Stop(reason)
    }

    /// After a stall: the gap counts neither as silence nor as a release.
    pub(super) fn restart_windows(&mut self, now: DateTime<Utc>) {
        self.last_speech = now;
        if self.hold.released_since.is_some() {
            self.hold.released_since = Some(now);
        }
    }

    /// Takes in one tick's speech and the meeting app's hold.
    pub(super) fn observe(&mut self, observation: &Observation, now: DateTime<Utc>) {
        if observation.paused || observation.heard.mic || observation.heard.system {
            self.last_speech = now;
        }
        match observation.app_holds_mic {
            Some(true) => self.hold.held(now),
            Some(false) => self.hold.released(now, observation.heard.system),
            None => {}
        }
    }

    /// The decision for this tick.
    pub(super) fn decide(&mut self, observation: &Observation, now: DateTime<Utc>) -> Decision {
        if let Some(ending) = self.ending {
            if observation.paused || !self.is_due(ending.reason, observation, now) {
                self.ending = None;
                return Decision::Withdraw;
            }
            if now >= ending.stop_at {
                return self.stop(ending.reason);
            }
            return Decision::None;
        }
        if observation.paused {
            return Decision::None;
        }
        let Some(reason) = NOTICED
            .into_iter()
            .find(|reason| self.is_due(*reason, observation, now))
        else {
            return Decision::None;
        };
        let notice = EndingNotice {
            reason,
            stop_at: now + ENDING_NOTICE,
        };
        self.ending = Some(notice);
        Decision::Ending(notice)
    }

    /// Whether `reason`'s rule holds now.
    fn is_due(&self, reason: StopReason, observation: &Observation, now: DateTime<Utc>) -> bool {
        let quiet = now - self.last_speech;
        match reason {
            StopReason::MicReleased => {
                observation.app_holds_mic == Some(false)
                    && self.hold.released_for(now) >= MIC_RELEASE
            }
            StopReason::NoSpeech => quiet >= NO_SPEECH,
            StopReason::CalendarEnded => {
                !self.calendar_kept
                    && quiet >= CALENDAR_QUIET
                    && observation
                        .calendar_end
                        .is_some_and(|end| now >= end + CALENDAR_GRACE)
            }
            StopReason::Sleep | StopReason::DiskFull => false,
        }
    }
}

impl Hold {
    /// The app holds the microphone at `now`.
    fn held(&mut self, now: DateTime<Utc>) {
        let since = *self.since.get_or_insert(now);
        if now - since >= MIN_OWNERSHIP {
            self.owned = true;
        }
        self.released_since = None;
        self.kept = false;
    }

    /// The app does not hold the microphone at `now`; speech on the call
    /// says the meeting goes on, so the wait starts again.
    fn released(&mut self, now: DateTime<Utc>, call_speaking: bool) {
        self.since = None;
        if !self.owned || self.kept {
            return;
        }
        if call_speaking || self.released_since.is_none() {
            self.released_since = Some(now);
        }
    }

    /// How long a counted release has lasted (zero when there is none).
    fn released_for(&self, now: DateTime<Utc>) -> chrono::TimeDelta {
        self.released_since
            .filter(|_| self.owned && !self.kept)
            .map_or(chrono::TimeDelta::zero(), |since| now - since)
    }
}
