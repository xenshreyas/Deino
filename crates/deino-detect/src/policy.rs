//! [`DetectionPolicy`]: when a candidate becomes a prompt, and when the
//! prompt goes away. A pure state machine over an injected [`Clock`].
//!
//! ```text
//!           candidate            held for its dwell
//!   Idle ─────────────▶ Pending ─────────────────▶ Prompting ──┐
//!    ▲  ◀─────────────── (gone or changed)           │  │       │ app let go for END_GRACE
//!    │                                               │  │       ▼
//!    │     Take notes / Ignore / Don't record / 30 s │  │     (End) ──▶ Idle
//!    │                                               ▼  │
//!    └──────── meeting gone for END_GRACE ──────── Handled ◀─ recording started
//! ```
//!
//! - **Dwell:** a candidate must hold steady before it prompts (Muesli's
//!   stability rule): [`SHORT_DWELL`] for strong evidence, [`SUSTAINED_DWELL`]
//!   for an unknown app (anarlog's 15 s microphone hold).
//! - **Handled:** once the user acted on a prompt, or recording started, the
//!   same meeting never prompts again until it ends.
//! - **Cooldown:** "Ignore" (and "Don't record") silences that app for
//!   [`DISMISS_COOLDOWN`] (anarlog's 10 minutes).
//! - **Suppressed while recording** and while detection is off.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{BundleId, Clock};

use crate::event::DetectionId;
use crate::snapshot::{Candidate, CandidateKey, Dwell};
use steps::{OnScreen, Seen};

/// How long strong evidence must hold: the prompt lands within the card's 3 s.
pub const SHORT_DWELL: TimeDelta = TimeDelta::milliseconds(2_500);

/// How long an unknown app must hold the microphone before it prompts.
pub const SUSTAINED_DWELL: TimeDelta = TimeDelta::seconds(15);

/// How long a meeting may vanish before it counts as ended: rides out a
/// microphone flicker or a missed Accessibility read.
pub const END_GRACE: TimeDelta = TimeDelta::seconds(2);

/// An unanswered prompt closes after this long.
pub const PROMPT_TIMEOUT: TimeDelta = TimeDelta::seconds(30);

/// A dismissed app stays quiet this long.
pub const DISMISS_COOLDOWN: TimeDelta = TimeDelta::minutes(10);

/// What the evaluation tells the outside world to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyAction {
    /// Show a prompt for this meeting.
    Prompt(DetectionId, Candidate),
    /// Close the prompt: it was handled, timed out, or recording started.
    Close(DetectionId),
    /// The prompted meeting ended: its app let go of the microphone.
    End(DetectionId),
}

/// What gates detection besides the evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gate {
    /// Detection is switched on.
    pub enabled: bool,
    /// Deino is recording, so nothing prompts.
    pub recording: bool,
}

/// How the user answered a prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// "Take notes", or the auto-start countdown ran out.
    TakeNotes,
    /// "Ignore" or "Don't record": the app cools down.
    Dismiss,
    /// "Always ignore this app" (the caller saves the ignore list).
    AlwaysIgnore,
}

#[derive(Clone, Debug)]
enum Phase {
    Idle,
    Pending {
        candidate: Candidate,
        since: DateTime<Utc>,
    },
    Prompting {
        id: DetectionId,
        candidate: Candidate,
        shown_at: DateTime<Utc>,
        missing_since: Option<DateTime<Utc>>,
    },
    Handled {
        key: CandidateKey,
        missing_since: Option<DateTime<Utc>>,
    },
}

/// The prompt state machine.
#[derive(Debug)]
pub struct DetectionPolicy {
    clock: Arc<dyn Clock>,
    phase: Phase,
    cooldowns: HashMap<BundleId, DateTime<Utc>>,
    last_id: u32,
}

impl DetectionPolicy {
    /// An idle policy reading time from `clock`.
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            phase: Phase::Idle,
            cooldowns: HashMap::new(),
            last_id: 0,
        }
    }

    /// Advances the state machine with the latest candidate.
    pub fn evaluate(&mut self, candidate: Option<Candidate>, gate: Gate) -> Vec<PolicyAction> {
        let now = self.clock.now();
        self.cooldowns.retain(|_, until| *until > now);
        if !gate.enabled || gate.recording {
            return self.suppress(candidate.filter(|_| gate.enabled));
        }
        let candidate = candidate.filter(|c| !self.cooldowns.contains_key(&c.app.bundle_id));
        let seen = Seen { candidate, now };
        match std::mem::replace(&mut self.phase, Phase::Idle) {
            Phase::Idle => self.observe(seen),
            Phase::Pending { candidate, since } => self.pending(&candidate.key, since, seen),
            Phase::Prompting {
                id,
                candidate,
                shown_at,
                missing_since,
            } => {
                let shown = OnScreen {
                    id,
                    candidate,
                    shown_at,
                    missing_since,
                };
                self.prompting(shown, seen)
            }
            Phase::Handled { key, missing_since } => self.handled(key, missing_since, seen),
        }
    }

    /// Records the user's answer to prompt `id`; `None` when it already closed.
    pub fn answer(&mut self, id: DetectionId, answer: Answer) -> Option<Candidate> {
        let Phase::Prompting {
            id: shown_id,
            candidate,
            ..
        } = &self.phase
        else {
            return None;
        };
        if *shown_id != id {
            return None;
        }
        let candidate = candidate.clone();
        if answer == Answer::Dismiss {
            self.cooldowns.insert(
                candidate.app.bundle_id.clone(),
                self.clock.now() + DISMISS_COOLDOWN,
            );
        }
        self.phase = Phase::Handled {
            key: candidate.key.clone(),
            missing_since: None,
        };
        Some(candidate)
    }

    /// The prompt on screen, if any.
    pub fn prompt(&self) -> Option<(DetectionId, &Candidate)> {
        match &self.phase {
            Phase::Prompting { id, candidate, .. } => Some((*id, candidate)),
            _ => None,
        }
    }

    /// When the state changes with no new evidence: a dwell ends, a prompt
    /// times out, or a vanished meeting's grace runs out.
    pub fn next_deadline(&self) -> Option<DateTime<Utc>> {
        match &self.phase {
            Phase::Idle => None,
            Phase::Pending { candidate, since } => Some(*since + dwell(candidate)),
            Phase::Prompting {
                shown_at,
                missing_since,
                ..
            } => {
                let timeout = *shown_at + PROMPT_TIMEOUT;
                Some(missing_since.map_or(timeout, |since| timeout.min(since + END_GRACE)))
            }
            Phase::Handled { missing_since, .. } => missing_since.map(|since| since + END_GRACE),
        }
    }
}

fn dwell(candidate: &Candidate) -> TimeDelta {
    match candidate.dwell {
        Dwell::Short => SHORT_DWELL,
        Dwell::Sustained => SUSTAINED_DWELL,
    }
}

mod steps;
#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
