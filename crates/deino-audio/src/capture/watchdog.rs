//! The stall watchdog: when to restart a track's source, with backoff.
//!
//! Each supervisor tick feeds a track's [`TrackSample`] (frames delivered so
//! far and whether the device reported a failure) into its [`TrackWatch`].
//! A track whose device failed, or that delivered nothing for
//! [`WatchPolicy::stall_after`], opens a recovery *episode* (muesli's
//! system-audio watchdog, reimplemented): restart now, then after
//! [`BACKOFF`] steps, at most [`MAX_ATTEMPTS`] times; then give up for
//! [`COOLDOWN`] and try again. Two healthy ticks in a row close the episode.
//!
//! Both sources are clock-driven: the cpal input stream and the tap's
//! aggregate device (started with `tap_auto_start = false`) call back every
//! IO cycle even in silence, so frozen frames are a real stall on either
//! track (see ADR 0026 for the measurement).

use std::time::Duration;

use crate::units::{Frames, HostTime};

/// Delays between recovery attempts after the first (muesli: 2 s, then 5 s).
pub const BACKOFF: [Duration; 2] = [Duration::from_secs(2), Duration::from_secs(5)];

/// Attempts per episode before giving up for a while.
pub const MAX_ATTEMPTS: u8 = 4;

/// How long the watchdog waits after giving up before trying again.
pub const COOLDOWN: Duration = Duration::from_secs(15);

/// Consecutive healthy ticks that close an episode.
pub const HEALTHY_TICKS: u8 = 2;

/// How one track is watched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WatchPolicy {
    /// No new frames for this long is a stall.
    pub stall_after: Duration,
}

impl WatchPolicy {
    /// The microphone: a clock-driven input stream that is late by 2 s has stalled.
    pub const MICROPHONE: Self = Self {
        stall_after: Duration::from_secs(2),
    };

    /// The system tap: a little more slack for the aggregate device to attach.
    pub const SYSTEM: Self = Self {
        stall_after: Duration::from_secs(3),
    };
}

/// One tick's view of a track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackSample {
    /// Frames delivered so far.
    pub frames: Frames,
    /// False when the device reported an error or disappeared.
    pub alive: bool,
}

/// What the supervisor should do for a track this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchAction {
    /// Nothing.
    None,
    /// Restart the source; `attempt` counts from 1 within the episode.
    Restart {
        /// The attempt number.
        attempt: u8,
    },
    /// Stop trying for [`COOLDOWN`]; the track records silence meanwhile.
    GiveUp,
    /// The track is healthy again; the episode is over.
    Recovered,
}

/// A recovery episode in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Episode {
    attempts: u8,
    next_at: HostTime,
    healthy: u8,
}

/// One track's watchdog state.
#[derive(Clone, Copy, Debug)]
pub struct TrackWatch {
    policy: WatchPolicy,
    frames: Frames,
    changed_at: HostTime,
    episode: Option<Episode>,
}

impl TrackWatch {
    /// A watch that considers the track fresh at `now`.
    pub fn new(policy: WatchPolicy, now: HostTime) -> Self {
        Self {
            policy,
            frames: Frames::ZERO,
            changed_at: now,
            episode: None,
        }
    }

    /// Forgets any stall (after a deliberate stop such as a pause).
    pub fn reset(&mut self, now: HostTime) {
        self.changed_at = now;
        self.episode = None;
    }

    /// Whether a recovery episode is open.
    pub fn is_recovering(&self) -> bool {
        self.episode.is_some()
    }

    /// Feeds one tick; returns what to do.
    pub fn observe(&mut self, now: HostTime, sample: TrackSample) -> WatchAction {
        if sample.frames != self.frames {
            self.frames = sample.frames;
            self.changed_at = now;
        }
        let healthy = sample.alive && now.since(self.changed_at) < self.policy.stall_after;
        match self.episode {
            None if healthy => WatchAction::None,
            None => self.open(now),
            Some(_) if healthy => self.heal(),
            Some(episode) => self.retry(now, episode),
        }
    }

    /// Opens an episode with an immediate first attempt.
    fn open(&mut self, now: HostTime) -> WatchAction {
        self.episode = Some(Episode {
            attempts: 1,
            next_at: now.plus(BACKOFF[0]),
            healthy: 0,
        });
        WatchAction::Restart { attempt: 1 }
    }

    /// Counts a healthy tick; closes the episode after enough of them.
    fn heal(&mut self) -> WatchAction {
        let Some(episode) = self.episode.as_mut() else {
            return WatchAction::None;
        };
        episode.healthy += 1;
        if episode.healthy < HEALTHY_TICKS {
            return WatchAction::None;
        }
        self.episode = None;
        WatchAction::Recovered
    }

    /// Schedules the next attempt, or gives up for [`COOLDOWN`].
    fn retry(&mut self, now: HostTime, mut episode: Episode) -> WatchAction {
        episode.healthy = 0;
        if now < episode.next_at {
            self.episode = Some(episode);
            return WatchAction::None;
        }
        if episode.attempts >= MAX_ATTEMPTS {
            episode.attempts = 0;
            episode.next_at = now.plus(COOLDOWN);
            self.episode = Some(episode);
            return WatchAction::GiveUp;
        }
        episode.attempts += 1;
        episode.next_at = now.plus(BACKOFF[1]);
        self.episode = Some(episode);
        WatchAction::Restart {
            attempt: episode.attempts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    struct Rig {
        watch: TrackWatch,
        now: HostTime,
        frames: u64,
    }

    impl Rig {
        fn new() -> Self {
            Self {
                watch: TrackWatch::new(WatchPolicy::MICROPHONE, HostTime(1)),
                now: HostTime(1),
                frames: 0,
            }
        }

        /// Advances one second; `flowing` delivers a second of audio.
        fn tick(&mut self, flowing: bool, alive: bool) -> WatchAction {
            self.now = self.now.plus(SECOND);
            if flowing {
                self.frames += 48_000;
            }
            let sample = TrackSample {
                frames: Frames(self.frames),
                alive,
            };
            self.watch.observe(self.now, sample)
        }
    }

    #[test]
    fn a_flowing_track_is_left_alone() {
        let mut rig = Rig::new();
        for _ in 0..10 {
            assert_eq!(rig.tick(true, true), WatchAction::None);
        }
    }

    #[test]
    fn a_dead_device_is_restarted_at_once() {
        let mut rig = Rig::new();
        assert_eq!(rig.tick(true, false), WatchAction::Restart { attempt: 1 });
    }

    #[test]
    fn a_stall_backs_off_gives_up_then_cools_down() {
        let mut rig = Rig::new();
        let actions: Vec<WatchAction> = (0..20).map(|_| rig.tick(false, true)).collect();
        let restarts: Vec<usize> = actions
            .iter()
            .enumerate()
            .filter(|(_, action)| matches!(action, WatchAction::Restart { .. }))
            .map(|(second, _)| second + 1)
            .collect();
        // Stalled at 2 s, then +2 s, +5 s, +5 s; given up at +5 s more.
        assert_eq!(restarts, [2, 4, 9, 14]);
        assert_eq!(actions[18], WatchAction::GiveUp);
    }

    #[test]
    fn two_healthy_ticks_close_an_episode() {
        let mut rig = Rig::new();
        assert_eq!(rig.tick(true, false), WatchAction::Restart { attempt: 1 });
        assert_eq!(rig.tick(true, true), WatchAction::None);
        assert_eq!(rig.tick(true, true), WatchAction::Recovered);
        assert!(!rig.watch.is_recovering());
    }

    #[test]
    fn a_reset_forgets_a_deliberate_silence() {
        let mut rig = Rig::new();
        rig.now = rig.now.plus(SECOND * 30);
        rig.watch.reset(rig.now);
        assert_eq!(rig.tick(true, true), WatchAction::None);
    }
}
