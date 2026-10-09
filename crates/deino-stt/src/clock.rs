//! The monotonic clock behind latency and real-time-factor measurements.
//!
//! Wall-clock reads are banned outside sanctioned clocks (`clippy.toml`), so
//! the stream metrics read elapsed time through [`MonotonicClock`]: the app
//! passes [`SystemMonotonic`], tests a [`FakeMonotonic`] they move by hand.

use std::fmt::Debug;
use std::time::{Duration, Instant};

/// Elapsed time since an arbitrary, fixed origin.
pub trait MonotonicClock: Debug + Send + Sync {
    /// Time since the clock's origin.
    fn now(&self) -> Duration;
}

/// The real monotonic clock.
#[derive(Clone, Copy, Debug)]
pub struct SystemMonotonic {
    origin: Instant,
}

impl SystemMonotonic {
    /// A clock whose origin is now.
    #[allow(
        clippy::disallowed_methods,
        reason = "SystemMonotonic is the one sanctioned reader of the monotonic clock in deino-stt"
    )]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemMonotonic {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for SystemMonotonic {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeMonotonic;

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::sync::{Mutex, PoisonError};
    use std::time::Duration;

    use super::MonotonicClock;

    /// A monotonic clock that only moves when a test moves it.
    #[derive(Debug, Default)]
    pub struct FakeMonotonic {
        now: Mutex<Duration>,
    }

    impl FakeMonotonic {
        /// Moves the clock forward by `delta`.
        pub fn advance(&self, delta: Duration) {
            *self.now.lock().unwrap_or_else(PoisonError::into_inner) += delta;
        }
    }

    impl MonotonicClock for FakeMonotonic {
        fn now(&self) -> Duration {
            *self.now.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }
}
