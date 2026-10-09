//! The `HostClock` port: the monotonic clock audio timing is measured on.
//!
//! `deino_domain::Clock` is the wall clock (for `started_at` stamps). Audio
//! needs a monotonic clock in the same domain as Core Audio's
//! `AudioTimeStamp::mHostTime`, so capture stamps, the timeline and stall
//! detection all agree. Tests drive time with [`FakeHostClock`] and never sleep.

use std::fmt::Debug;

use crate::units::HostTime;

/// A source of monotonic host time.
pub trait HostClock: Debug + Send + Sync {
    /// The current host time.
    fn now(&self) -> HostTime;
}

/// The real host clock: `mach_absolute_time` on macOS, converted to
/// nanoseconds with the timebase (`ns = ticks × numer / denom`; inverting the
/// ratio is a known bug that inflates time about 41× on Apple Silicon).
#[derive(Clone, Copy, Debug)]
pub struct SystemHostClock {
    #[cfg(target_os = "macos")]
    numer: u64,
    #[cfg(target_os = "macos")]
    denom: u64,
    #[cfg(not(target_os = "macos"))]
    origin: std::time::Instant,
}

impl SystemHostClock {
    /// Reads the timebase once; the clock is cheap to call afterwards.
    #[cfg(target_os = "macos")]
    pub fn new() -> Self {
        let timebase = cidre::mach::TimeBaseInfo::new();
        Self {
            numer: u64::from(timebase.numer.max(1)),
            denom: u64::from(timebase.denom.max(1)),
        }
    }

    /// Anchors the clock at process start on platforms without mach time.
    #[cfg(not(target_os = "macos"))]
    #[allow(
        clippy::disallowed_methods,
        reason = "SystemHostClock is the audio crate's one sanctioned reader of the monotonic clock"
    )]
    pub fn new() -> Self {
        Self {
            origin: std::time::Instant::now(),
        }
    }
}

impl Default for SystemHostClock {
    fn default() -> Self {
        Self::new()
    }
}

impl HostClock for SystemHostClock {
    #[cfg(target_os = "macos")]
    fn now(&self) -> HostTime {
        let ticks = u128::from(cidre::mach::abs_time());
        let nanos = ticks * u128::from(self.numer) / u128::from(self.denom);
        HostTime(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    #[cfg(not(target_os = "macos"))]
    fn now(&self) -> HostTime {
        HostTime(u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX))
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeHostClock;

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use super::HostClock;
    use crate::units::HostTime;

    /// A host clock that only moves when a test moves it.
    #[derive(Debug, Default)]
    pub struct FakeHostClock {
        nanos: AtomicU64,
    }

    impl FakeHostClock {
        /// Creates a clock frozen at `start`.
        pub fn new(start: HostTime) -> Self {
            Self {
                nanos: AtomicU64::new(start.0),
            }
        }

        /// Moves the clock forward by `delta`.
        pub fn advance(&self, delta: Duration) {
            let nanos = u64::try_from(delta.as_nanos()).unwrap_or(u64::MAX);
            self.nanos.fetch_add(nanos, Ordering::SeqCst);
        }
    }

    impl HostClock for FakeHostClock {
        fn now(&self) -> HostTime {
            HostTime(self.nanos.load(Ordering::SeqCst))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn fake_host_clock_stays_put_until_advanced() {
        let clock = FakeHostClock::new(HostTime(100));
        assert_eq!(clock.now(), HostTime(100));
        clock.advance(Duration::from_millis(5));
        assert_eq!(clock.now(), HostTime(5_000_100));
    }

    #[test]
    fn system_host_clock_never_goes_backwards() {
        let clock = SystemHostClock::new();
        let first = clock.now();
        assert!(clock.now() >= first);
    }
}
