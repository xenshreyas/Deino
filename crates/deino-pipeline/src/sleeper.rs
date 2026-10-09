//! Waiting until a time on the injected clock.
//!
//! Retry backoff and the cancellation grace period are deadlines computed
//! from the [`Clock`]. The runner waits for them through the [`Sleeper`]
//! port, so tests drive time with [`FakeSleeper`] (which moves a
//! `FakeClock` to the deadline at once) and never sleep.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::Clock;

/// Waits until a deadline.
#[async_trait]
pub trait Sleeper: Send + Sync {
    /// Returns once the clock reads `deadline` or later.
    async fn sleep_until(&self, deadline: DateTime<Utc>);
}

/// The real [`Sleeper`]: tokio timers, measured against the injected clock.
#[derive(Clone, Debug)]
pub struct ClockSleeper {
    clock: Arc<dyn Clock>,
}

impl ClockSleeper {
    /// A sleeper whose deadlines are read from `clock`.
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self { clock }
    }
}

#[async_trait]
impl Sleeper for ClockSleeper {
    async fn sleep_until(&self, deadline: DateTime<Utc>) {
        // Loops because a timer can wake before the wall clock agrees
        // (sleep, clock adjustments).
        while let Ok(remaining) = (deadline - self.clock.now()).to_std() {
            if remaining.is_zero() {
                return;
            }
            tokio::time::sleep(remaining).await;
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeSleeper;

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::sync::{Arc, Mutex, PoisonError};

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use deino_domain::{Clock, FakeClock};

    use super::Sleeper;

    /// A [`Sleeper`] for tests: records each deadline and moves the fake
    /// clock there at once, or, when stalled, never returns.
    #[derive(Debug)]
    pub struct FakeSleeper {
        clock: Arc<FakeClock>,
        stalled: bool,
        deadlines: Mutex<Vec<DateTime<Utc>>>,
    }

    impl FakeSleeper {
        /// A sleeper that jumps `clock` to every deadline.
        pub fn new(clock: Arc<FakeClock>) -> Self {
            Self {
                clock,
                stalled: false,
                deadlines: Mutex::default(),
            }
        }

        /// A sleeper that records deadlines but never wakes, for tests that
        /// interrupt a wait.
        pub fn stalled(clock: Arc<FakeClock>) -> Self {
            Self {
                stalled: true,
                ..Self::new(clock)
            }
        }

        /// Every deadline waited for, in order.
        pub fn deadlines(&self) -> Vec<DateTime<Utc>> {
            self.deadlines
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl Sleeper for FakeSleeper {
        async fn sleep_until(&self, deadline: DateTime<Utc>) {
            self.deadlines
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(deadline);
            if self.stalled {
                std::future::pending::<()>().await;
            }
            let now = self.clock.now();
            if deadline > now {
                self.clock.advance(deadline - now);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use deino_domain::SystemClock;

    use super::*;

    #[tokio::test]
    async fn a_past_deadline_returns_at_once() {
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let sleeper = ClockSleeper::new(Arc::clone(&clock));

        sleeper
            .sleep_until(clock.now() - TimeDelta::seconds(1))
            .await;
    }
}
