//! The `Clock` port: the only way core logic reads the current time.
//!
//! Services take an `Arc<dyn Clock>` instead of calling `Utc::now()`, so tests
//! drive time with [`FakeClock`] and never sleep. `clippy.toml` bans the
//! wall-clock functions everywhere except [`SystemClock`].

use std::fmt::Debug;

use chrono::{DateTime, Utc};

/// A source of the current wall-clock time.
pub trait Clock: Debug + Send + Sync {
    /// The current time in UTC.
    fn now(&self) -> DateTime<Utc>;
}

/// The real clock, wired in by the app's composition root.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    #[allow(
        clippy::disallowed_methods,
        reason = "SystemClock is the one sanctioned reader of the wall clock"
    )]
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeClock;

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::sync::{Mutex, PoisonError};

    use chrono::{DateTime, TimeDelta, Utc};

    use super::Clock;

    /// A clock that only moves when a test moves it.
    #[derive(Debug)]
    pub struct FakeClock {
        now: Mutex<DateTime<Utc>>,
    }

    impl FakeClock {
        /// Creates a clock frozen at `start`.
        pub fn new(start: DateTime<Utc>) -> Self {
            Self {
                now: Mutex::new(start),
            }
        }

        /// Moves the clock forward by `delta`.
        pub fn advance(&self, delta: TimeDelta) {
            let mut now = self.now.lock().unwrap_or_else(PoisonError::into_inner);
            *now += delta;
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            *self.now.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone};

    use super::*;

    #[test]
    fn fake_clock_stays_put_until_advanced() {
        let start = Utc.with_ymd_and_hms(2026, 10, 8, 9, 0, 0).unwrap();
        let clock = FakeClock::new(start);

        assert_eq!(clock.now(), start);
        clock.advance(TimeDelta::minutes(5));
        assert_eq!(clock.now(), start + TimeDelta::minutes(5));
    }
}
