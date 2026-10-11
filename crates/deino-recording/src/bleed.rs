//! The bleed holdback: the text-side net behind the echo canceller.
//!
//! When the call plays through speakers, echo the canceller leaves behind
//! can still be transcribed on the Me lane, duplicating what Them said. The
//! holdback is OpenWhispr's `meetingMicHoldback` (MIT), ported, with one
//! change to *when* a segment is judged:
//!
//! - a Me segment is **held until the Them lane has transcribed past it**:
//!   until Them's transcript watermark ([`deino_stt::SessionSink::progress`])
//!   reaches its end plus [`HoldbackPolicy::settle`]. OpenWhispr holds for a
//!   fixed 3 s of wall time, which judges too early whenever the Them lane
//!   lags (a loaded machine, a recovering worker) and lets echo through; the
//!   watermark makes the verdict depend on the audio, not on scheduling;
//! - it is then **dropped only on a text match** with Them text from the
//!   same moment ([`overlap::overlaps`], similarity above 0.6, against up to
//!   three adjacent Them segments merged, plus Them's open segment), and only
//!   if Them was speaking around it (a voice span or text within
//!   [`HoldbackPolicy::risk_margin`]); otherwise it is kept. A segment that
//!   Them's text already proves to be echo is dropped at once.
//! - A Them lane that stops reporting progress (its engine is down) cannot
//!   hold a Me segment longer than [`HoldbackPolicy::max_wait`] of wall time.
//!
//! Audio correlation alone never drops anything: OpenWhispr's field data shows
//! real double-talk correlating 0.73–0.81 with the system audio. The
//! holdback is pure; the transcript router feeds it and acts on its verdicts.

mod evidence;
pub mod overlap;

use std::collections::VecDeque;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use deino_stt::Millis;

use self::evidence::ThemEvidence;
use crate::vad::VoiceSpan;

/// The holdback's thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoldbackPolicy {
    /// How far past a Me segment's end the Them lane must have transcribed
    /// before the segment is judged.
    pub settle: Millis,
    /// The longest a segment waits, in wall time, for a Them lane that has
    /// stopped making progress.
    pub max_wait: Duration,
    /// How far apart in time a Them segment may start and still be compared.
    pub window: Millis,
    /// Adjacent Them segments merged into one candidate text.
    pub merge: usize,
    /// Them activity this close to a Me segment makes it risky.
    pub risk_margin: Millis,
    /// Them history older than this behind the newest Me segment is forgotten.
    pub memory: Millis,
}

impl Default for HoldbackPolicy {
    fn default() -> Self {
        Self {
            settle: Millis(1_000),
            max_wait: Duration::from_secs(20),
            window: Millis(6_000),
            merge: 3,
            risk_margin: Millis(500),
            memory: Millis(30_000),
        }
    }
}

/// What to do with a Me segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict<T> {
    /// Commit it.
    Keep(T),
    /// It is the call's audio heard by the microphone: discard it.
    Drop(T),
}

/// A held Me segment.
#[derive(Debug)]
struct Held<T> {
    start: Millis,
    end: Millis,
    tokens: Vec<String>,
    /// Judged once Them's watermark reaches this.
    due: Millis,
    /// Judged at this wall time whatever Them's watermark says.
    give_up_at: DateTime<Utc>,
    payload: T,
}

/// Held Me segments and the Them evidence they are checked against.
#[derive(Debug)]
pub struct Holdback<T> {
    policy: HoldbackPolicy,
    held: VecDeque<Held<T>>,
    them: ThemEvidence,
    /// Them's transcript watermark.
    them_through: Millis,
}

/// A Me segment offered to the holdback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer<T> {
    /// When it starts.
    pub start: Millis,
    /// When it ends.
    pub end: Millis,
    /// What it says.
    pub text: String,
    /// What the caller gets back in the verdict.
    pub payload: T,
}

impl<T> Holdback<T> {
    /// An empty holdback.
    pub fn new(policy: HoldbackPolicy) -> Self {
        Self {
            policy,
            held: VecDeque::new(),
            them: ThemEvidence::default(),
            them_through: Millis::ZERO,
        }
    }

    /// Them started speaking at `at`.
    pub fn them_voice_started(&mut self, at: Millis) {
        self.them.voice_started(at);
    }

    /// Records that Them was speaking during `span`.
    pub fn them_voice(&mut self, span: VoiceSpan) {
        self.them.voice(span);
    }

    /// Records Them's open (still growing) segment; drops held Me segments
    /// it proves to be echo.
    pub fn them_live(&mut self, start: Millis, text: &str) -> Vec<Verdict<T>> {
        self.them.live(start, text);
        self.drop_proven()
    }

    /// Records a closed Them segment; drops held Me segments it proves to be echo.
    pub fn them_final(&mut self, start: Millis, end: Millis, text: &str) -> Vec<Verdict<T>> {
        self.them.closed(start, end, text);
        self.drop_proven()
    }

    /// The Them lane has transcribed everything before `through`: judges
    /// every held segment that was waiting for it.
    pub fn them_progress(&mut self, through: Millis) -> Vec<Verdict<T>> {
        self.them_through = self.them_through.max(through);
        let watermark = self.them_through;
        self.release(|held| held.due <= watermark)
    }

    /// Offers a Me segment at wall time `now`: judged at once when Them's
    /// text already proves it echo or the Them lane is already past it,
    /// held otherwise.
    pub fn offer(&mut self, offer: Offer<T>, now: DateTime<Utc>) -> Option<Verdict<T>> {
        self.them
            .forget_before(offer.start.saturating_sub(self.policy.memory));
        let wait = TimeDelta::from_std(self.policy.max_wait).unwrap_or(TimeDelta::MAX);
        let held = Held {
            start: offer.start,
            end: offer.end,
            tokens: overlap::tokens(&offer.text),
            due: offer.end + self.policy.settle,
            give_up_at: now + wait,
            payload: offer.payload,
        };
        if held.due <= self.them_through || self.proven_echo(&held) {
            return Some(self.judge(held));
        }
        self.held.push_back(held);
        None
    }

    /// Judges every held segment whose wall-time wait has run out.
    pub fn tick(&mut self, now: DateTime<Utc>) -> Vec<Verdict<T>> {
        self.release(|held| held.give_up_at <= now)
    }

    /// Judges every held segment now (the recording is stopping).
    pub fn drain(&mut self) -> Vec<Verdict<T>> {
        self.release(|_| true)
    }

    /// Whether any segment is held.
    pub fn is_holding(&self) -> bool {
        !self.held.is_empty()
    }

    /// Judges and removes the held segments `ready` picks, oldest first.
    fn release(&mut self, ready: impl Fn(&Held<T>) -> bool) -> Vec<Verdict<T>> {
        let (due, waiting): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.held).into_iter().partition(ready);
        self.held = waiting.into();
        due.into_iter().map(|held| self.judge(held)).collect()
    }

    fn judge(&self, held: Held<T>) -> Verdict<T> {
        if self.proven_echo(&held) {
            Verdict::Drop(held.payload)
        } else {
            Verdict::Keep(held.payload)
        }
    }

    /// Them was speaking around the segment and said the same words.
    fn proven_echo(&self, held: &Held<T>) -> bool {
        self.them.is_risky(&self.policy, held.start, held.end)
            && self.them.matches(&self.policy, held.start, &held.tokens)
    }

    /// Drops every held segment that Them's text now proves to be echo.
    fn drop_proven(&mut self) -> Vec<Verdict<T>> {
        let (proven, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.held)
            .into_iter()
            .partition(|held| self.proven_echo(held));
        self.held = waiting.into();
        proven
            .into_iter()
            .map(|held| Verdict::Drop(held.payload))
            .collect()
    }
}

#[cfg(test)]
#[path = "bleed_tests.rs"]
mod tests;
