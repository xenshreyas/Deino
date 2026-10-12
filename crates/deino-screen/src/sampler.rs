//! [`SnapshotSampler`]: which frames become snapshots.
//!
//! Every [`SnapshotPolicy::every`] of recording time (10 s, Granola's
//! cadence) the sampler looks at one frame. It keeps that frame when it is
//! the first, or when its [`PHash`] differs from the last *kept* one by
//! more than [`SnapshotPolicy::threshold`] bits; otherwise the screen has
//! not changed enough to be worth another picture. Comparing with the last
//! kept frame (not the last seen one) means a slow scroll still produces a
//! snapshot once it has moved far enough. Pure, driven by the caller's
//! recording times.

use deino_domain::Millis;

use crate::phash::PHash;

/// When to look and how different a frame must be to keep it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotPolicy {
    /// Recording time between two looks.
    pub every: Millis,
    /// Kept when more than this many of the 64 hash bits changed.
    pub threshold: u32,
}

impl Default for SnapshotPolicy {
    /// Every 10 s; more than 10 bits (about 15 %) changed. Cursor moves,
    /// a ticking clock or a speaker's video tile stay below it; a new
    /// slide, page or app is far above it (`phash` tests).
    fn default() -> Self {
        Self {
            every: Millis(10_000),
            threshold: 10,
        }
    }
}

/// What the sampler says about a frame it looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Keep it: the first frame, or the screen changed.
    Keep,
    /// Drop it: too close to the last kept frame.
    Unchanged,
}

/// Decides which frames become snapshots (see the module docs).
#[derive(Clone, Debug)]
pub struct SnapshotSampler {
    policy: SnapshotPolicy,
    looked_at: Option<Millis>,
    kept: Option<PHash>,
}

impl SnapshotSampler {
    /// A sampler that has seen nothing yet.
    pub const fn new(policy: SnapshotPolicy) -> Self {
        Self {
            policy,
            looked_at: None,
            kept: None,
        }
    }

    /// Whether a frame at `at` is due a look (hashing costs a pass over the
    /// frame, so callers ask first).
    pub fn due(&self, at: Millis) -> bool {
        self.looked_at
            .is_none_or(|last| at.0 >= last.0.saturating_add(self.policy.every.0))
    }

    /// Looks at a frame at `at` hashed `hash`. A kept frame only counts
    /// once [`kept`](Self::kept) confirms it was saved.
    pub fn judge(&mut self, at: Millis, hash: PHash) -> Verdict {
        self.looked_at = Some(at);
        match self.kept {
            Some(kept) if kept.distance(hash) <= self.policy.threshold => Verdict::Unchanged,
            _ => Verdict::Keep,
        }
    }

    /// Records that the frame hashed `hash` was saved: later frames are
    /// compared with it.
    pub const fn kept(&mut self, hash: PHash) {
        self.kept = Some(hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sampler() -> SnapshotSampler {
        SnapshotSampler::new(SnapshotPolicy::default())
    }

    #[test]
    fn a_look_is_due_every_ten_seconds() {
        let mut sampler = sampler();
        assert!(sampler.due(Millis(0)));

        sampler.judge(Millis(1_000), PHash(0));

        assert!(!sampler.due(Millis(10_999)));
        assert!(sampler.due(Millis(11_000)));
    }

    #[test]
    fn the_first_frame_is_kept_and_a_similar_one_is_not() {
        let mut sampler = sampler();
        assert_eq!(sampler.judge(Millis(0), PHash(0)), Verdict::Keep);
        sampler.kept(PHash(0));

        // Ten bits changed: at the threshold, not above it.
        assert_eq!(
            sampler.judge(Millis(10_000), PHash(0x3FF)),
            Verdict::Unchanged
        );
        assert_eq!(sampler.judge(Millis(20_000), PHash(0x7FF)), Verdict::Keep);
    }

    #[test]
    fn frames_are_compared_with_the_last_kept_one_not_the_last_seen() {
        let mut sampler = sampler();
        sampler.judge(Millis(0), PHash(0));
        sampler.kept(PHash(0));

        // A slow scroll: each look moves 6 bits from the one before.
        assert_eq!(
            sampler.judge(Millis(10_000), PHash(0x3F)),
            Verdict::Unchanged
        );
        assert_eq!(sampler.judge(Millis(20_000), PHash(0xFFF)), Verdict::Keep);
    }

    #[test]
    fn a_frame_that_failed_to_save_is_not_the_reference() {
        let mut sampler = sampler();
        assert_eq!(sampler.judge(Millis(0), PHash(0)), Verdict::Keep);
        // Not confirmed with `kept`: the next look is judged against nothing.
        assert_eq!(sampler.judge(Millis(10_000), PHash(1)), Verdict::Keep);
    }
}
