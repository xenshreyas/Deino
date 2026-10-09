//! The transitions of [`DetectionPolicy`], one function per phase.

use chrono::{DateTime, Utc};

use super::{DetectionPolicy, END_GRACE, PROMPT_TIMEOUT, Phase, PolicyAction, dwell};
use crate::event::DetectionId;
use crate::snapshot::{Candidate, CandidateKey};

impl DetectionPolicy {
    /// Off or recording: close any prompt; a meeting seen now counts as handled.
    pub(super) fn suppress(&mut self, candidate: Option<Candidate>) -> Vec<PolicyAction> {
        let closed = self.prompt().map(|(id, _)| PolicyAction::Close(id));
        self.phase = match candidate {
            Some(candidate) => Phase::Handled {
                key: candidate.key,
                missing_since: None,
            },
            None => Phase::Idle,
        };
        closed.into_iter().collect()
    }

    /// From idle: a candidate starts its dwell.
    pub(super) fn observe(&mut self, seen: Seen) -> Vec<PolicyAction> {
        if let Some(candidate) = seen.candidate {
            self.phase = Phase::Pending {
                candidate,
                since: seen.now,
            };
        }
        Vec::new()
    }

    /// A candidate that held for its dwell becomes a prompt.
    pub(super) fn pending(
        &mut self,
        key: &CandidateKey,
        since: DateTime<Utc>,
        seen: Seen,
    ) -> Vec<PolicyAction> {
        let Some(candidate) = seen.candidate.as_ref().filter(|c| c.key == *key) else {
            return self.observe(seen);
        };
        let candidate = candidate.clone();
        if seen.now < since + dwell(&candidate) {
            self.phase = Phase::Pending { candidate, since };
            return Vec::new();
        }
        self.last_id += 1;
        let id = DetectionId(self.last_id);
        self.phase = Phase::Prompting {
            id,
            candidate: candidate.clone(),
            shown_at: seen.now,
            missing_since: None,
        };
        vec![PolicyAction::Prompt(id, candidate)]
    }

    /// A prompt times out, or ends once its meeting is gone for the grace.
    pub(super) fn prompting(&mut self, shown: OnScreen, seen: Seen) -> Vec<PolicyAction> {
        let OnScreen {
            id,
            candidate,
            shown_at,
            missing_since,
        } = shown;
        if let Some(current) = seen.candidate.as_ref().filter(|c| c.key == candidate.key) {
            if seen.now >= shown_at + PROMPT_TIMEOUT {
                self.phase = Phase::Handled {
                    key: candidate.key,
                    missing_since: None,
                };
                return vec![PolicyAction::Close(id)];
            }
            self.phase = Phase::Prompting {
                id,
                candidate: current.clone(),
                shown_at,
                missing_since: None,
            };
            return Vec::new();
        }
        let missing_since = missing_since.unwrap_or(seen.now);
        if seen.now < missing_since + END_GRACE {
            self.phase = Phase::Prompting {
                id,
                candidate,
                shown_at,
                missing_since: Some(missing_since),
            };
            return Vec::new();
        }
        let mut actions = vec![PolicyAction::End(id)];
        actions.extend(self.observe(seen));
        actions
    }

    /// A handled meeting stays quiet until it is gone for the grace.
    pub(super) fn handled(
        &mut self,
        key: CandidateKey,
        missing_since: Option<DateTime<Utc>>,
        seen: Seen,
    ) -> Vec<PolicyAction> {
        if seen.candidate.as_ref().is_some_and(|c| c.key == key) {
            self.phase = Phase::Handled {
                key,
                missing_since: None,
            };
            return Vec::new();
        }
        let missing_since = missing_since.unwrap_or(seen.now);
        if seen.now < missing_since + END_GRACE {
            self.phase = Phase::Handled {
                key,
                missing_since: Some(missing_since),
            };
            return Vec::new();
        }
        self.observe(seen)
    }
}

/// One evaluation's input: the candidate, if any, and the time.
pub(super) struct Seen {
    pub(super) candidate: Option<Candidate>,
    pub(super) now: DateTime<Utc>,
}

/// The prompt being shown, unpacked from [`Phase::Prompting`].
pub(super) struct OnScreen {
    pub(super) id: DetectionId,
    pub(super) candidate: Candidate,
    pub(super) shown_at: DateTime<Utc>,
    pub(super) missing_since: Option<DateTime<Utc>>,
}
