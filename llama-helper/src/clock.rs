//! Time for the sidecar: the idle timer, the sampler seed and timings.
//!
//! The app reads time through the `deino_domain::Clock` port, but this is a
//! separate process without it, so the two functions here are the only
//! readers of the system clocks. [`IdleTimer`] keeps its decision pure
//! ([`IdleTimer::expired_at`]) so tests never depend on the real clock.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Time since the Unix epoch, or zero if the system clock is set before it.
#[allow(
    clippy::disallowed_methods,
    reason = "a separate process without the app's Clock port"
)]
pub fn since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

/// The current monotonic instant, for measuring how long work takes.
#[allow(
    clippy::disallowed_methods,
    reason = "a separate process without the app's Clock port"
)]
pub fn monotonic_now() -> Instant {
    Instant::now()
}

/// A seed for the sampler's random distribution: the low 32 bits of the
/// milliseconds since the epoch, as the sidecar has always used.
pub fn sampler_seed() -> u32 {
    let millis = since_epoch().as_millis() % (1_u128 << 32);
    u32::try_from(millis).unwrap_or_default()
}

/// Tracks the last request so the sidecar can exit when the app forgets it.
#[derive(Debug, Clone, Copy)]
pub struct IdleTimer {
    timeout_secs: u64,
    last_activity_secs: u64,
}

impl IdleTimer {
    /// A timer that expires `timeout_secs` after the last activity, starting now.
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            timeout_secs,
            last_activity_secs: since_epoch().as_secs(),
        }
    }

    /// Records activity now.
    pub fn touch(&mut self) {
        self.last_activity_secs = since_epoch().as_secs();
    }

    /// Whether the sidecar has been idle for longer than the timeout.
    pub fn expired(&self) -> bool {
        self.expired_at(since_epoch().as_secs())
    }

    /// Whether the timer has expired at `now_secs` (seconds since the epoch).
    pub fn expired_at(&self, now_secs: u64) -> bool {
        now_secs.saturating_sub(self.last_activity_secs) > self.timeout_secs
    }
}

#[cfg(test)]
mod tests {
    use super::IdleTimer;

    fn timer(timeout_secs: u64, last_activity_secs: u64) -> IdleTimer {
        IdleTimer {
            timeout_secs,
            last_activity_secs,
        }
    }

    #[test]
    fn expires_only_after_the_timeout_has_fully_passed() {
        let idle = timer(300, 1_000);
        assert!(!idle.expired_at(1_000));
        assert!(!idle.expired_at(1_300));
        assert!(idle.expired_at(1_301));
    }

    #[test]
    fn a_clock_that_moves_backwards_never_expires() {
        assert!(!timer(0, 1_000).expired_at(10));
    }
}
