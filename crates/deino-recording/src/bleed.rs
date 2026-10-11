//! The bleed holdback: the text-side net behind the echo canceller.
//!
//! When the call plays through speakers, echo the canceller leaves behind
//! can still be transcribed on the Me lane, duplicating what Them said. The
//! holdback is OpenWhispr's `meetingMicHoldback` (MIT), ported:
//!
//! - a Me segment spoken while the call was speaking (a Them voice span or
//!   Them text within [`HoldbackPolicy::risk_margin`]) is *held* for
//!   [`HoldbackPolicy::hold`] instead of being committed;
//! - it is **dropped only on a text match** with Them text from the same
//!   moment ([`overlap::overlaps`], similarity above 0.6, against up to three
//!   adjacent Them segments merged, plus Them's open segment);
//! - otherwise it is released when its hold expires.
//!
//! Audio correlation alone never drops anything: OpenWhispr's field data shows
//! real double-talk correlating 0.73–0.81 with the system audio. The
//! holdback is pure; the transcript router feeds it and acts on its verdicts.

pub mod overlap;

use std::collections::VecDeque;
use std::time::Duration;

use deino_audio::HostTime;
use deino_stt::Millis;

use crate::vad::VoiceSpan;

/// The holdback's thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoldbackPolicy {
    /// How long a risky Me segment waits for Them text (OpenWhispr: 3 s).
    pub hold: Duration,
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
            hold: Duration::from_secs(3),
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
    tokens: Vec<String>,
    release_at: HostTime,
    payload: T,
}

/// A Them segment's time and words.
#[derive(Debug)]
struct ThemText {
    start: Millis,
    end: Millis,
    tokens: Vec<String>,
}

/// Held Me segments and the Them evidence they are checked against.
#[derive(Debug)]
pub struct Holdback<T> {
    policy: HoldbackPolicy,
    held: VecDeque<Held<T>>,
    them: VecDeque<ThemText>,
    them_open: Option<ThemText>,
    voices: VecDeque<VoiceSpan>,
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
            them: VecDeque::new(),
            them_open: None,
            voices: VecDeque::new(),
        }
    }

    /// Records that Them was speaking during `span`.
    pub fn them_voice(&mut self, span: VoiceSpan) {
        self.voices.push_back(span);
    }

    /// Records Them's open (still growing) segment; drops held Me segments
    /// it proves to be echo.
    pub fn them_live(&mut self, start: Millis, text: &str) -> Vec<Verdict<T>> {
        let tokens = overlap::tokens(text);
        self.them_open = (!tokens.is_empty()).then_some(ThemText {
            start,
            end: start,
            tokens,
        });
        self.drop_matches()
    }

    /// Records a closed Them segment; drops held Me segments it proves to be echo.
    pub fn them_final(&mut self, start: Millis, end: Millis, text: &str) -> Vec<Verdict<T>> {
        self.them_open = None;
        self.them.push_back(ThemText {
            start,
            end,
            tokens: overlap::tokens(text),
        });
        self.drop_matches()
    }

    /// Offers a Me segment at `now`: kept at once when the call was quiet,
    /// dropped at once when Them already said it, held otherwise.
    pub fn offer(&mut self, offer: Offer<T>, now: HostTime) -> Option<Verdict<T>> {
        self.forget_before(offer.start);
        let tokens = overlap::tokens(&offer.text);
        if !self.is_risky(offer.start, offer.end) {
            return Some(Verdict::Keep(offer.payload));
        }
        if self.matches_them(offer.start, &tokens) {
            return Some(Verdict::Drop(offer.payload));
        }
        self.held.push_back(Held {
            start: offer.start,
            tokens,
            release_at: now.plus(self.policy.hold),
            payload: offer.payload,
        });
        None
    }

    /// Releases (or drops) every held segment whose hold has expired.
    pub fn tick(&mut self, now: HostTime) -> Vec<Verdict<T>> {
        let mut verdicts = Vec::new();
        while self.held.front().is_some_and(|held| held.release_at <= now) {
            if let Some(held) = self.held.pop_front() {
                verdicts.push(self.judge(held));
            }
        }
        verdicts
    }

    /// Judges every held segment now (the recording is stopping).
    pub fn drain(&mut self) -> Vec<Verdict<T>> {
        let held: Vec<Held<T>> = self.held.drain(..).collect();
        held.into_iter().map(|held| self.judge(held)).collect()
    }

    /// Whether any segment is held.
    pub fn is_holding(&self) -> bool {
        !self.held.is_empty()
    }

    fn judge(&self, held: Held<T>) -> Verdict<T> {
        if self.matches_them(held.start, &held.tokens) {
            Verdict::Drop(held.payload)
        } else {
            Verdict::Keep(held.payload)
        }
    }

    /// Drops every held segment that Them's text now matches.
    fn drop_matches(&mut self) -> Vec<Verdict<T>> {
        let (dropped, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.held)
            .into_iter()
            .partition(|held| self.matches_them(held.start, &held.tokens));
        self.held = kept.into();
        dropped
            .into_iter()
            .map(|held| Verdict::Drop(held.payload))
            .collect()
    }

    /// Whether Them was active around `[start, end]`.
    fn is_risky(&self, start: Millis, end: Millis) -> bool {
        let margin = self.policy.risk_margin;
        let (from, to) = (start.saturating_sub(margin), end + margin);
        let near = |s: Millis, e: Millis| s <= to && e >= from;
        self.voices.iter().any(|v| near(v.start, v.end))
            || self.them.iter().any(|t| near(t.start, t.end))
            || self.them_open.as_ref().is_some_and(|t| t.start <= to)
    }

    /// Whether Them's text near `start` says what `tokens` says.
    fn matches_them(&self, start: Millis, tokens: &[String]) -> bool {
        let window = self.policy.window;
        let nearby: Vec<&ThemText> = self
            .them
            .iter()
            .chain(self.them_open.as_ref())
            .filter(|t| t.start.0.abs_diff(start.0) <= window.0)
            .collect();
        candidates(&nearby, self.policy.merge)
            .iter()
            .any(|text| overlap::overlaps(tokens, text))
    }

    /// Forgets Them history far behind `start`.
    fn forget_before(&mut self, start: Millis) {
        let horizon = start.saturating_sub(self.policy.memory);
        while self.them.front().is_some_and(|t| t.end < horizon) {
            self.them.pop_front();
        }
        while self.voices.front().is_some_and(|v| v.end < horizon) {
            self.voices.pop_front();
        }
    }
}

/// Each run of up to `merge` consecutive texts, joined (OpenWhispr's
/// `buildMergedCandidates`), so a Me segment spanning two Them segments
/// still matches.
fn candidates(texts: &[&ThemText], merge: usize) -> Vec<Vec<String>> {
    let mut merged = Vec::new();
    for start in 0..texts.len() {
        let mut joined: Vec<String> = Vec::new();
        for text in texts.iter().skip(start).take(merge.max(1)) {
            joined.extend(text.tokens.iter().cloned());
            merged.push(joined.clone());
        }
    }
    merged
}

#[cfg(test)]
#[path = "bleed_tests.rs"]
mod tests;
