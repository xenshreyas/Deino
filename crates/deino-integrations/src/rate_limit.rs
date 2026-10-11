//! [`RateLimiter`]: keeps each connection under its provider's pace.
//!
//! A generic cell rate algorithm (GCRA), one "theoretical arrival time" per
//! connection: a call is due once that time, less the burst allowance, has
//! passed. Reserving a slot happens under a lock; waiting for it happens
//! outside, through the [`Sleeper`] port, so tests never sleep. The
//! provider's own 429s are handled by the retry policy on top.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{Clock, ConnectionId};

use crate::{RateLimit, Sleeper};

/// Paces calls per connection.
pub struct RateLimiter {
    clock: Arc<dyn Clock>,
    sleeper: Arc<dyn Sleeper>,
    arrivals: Mutex<BTreeMap<ConnectionId, DateTime<Utc>>>,
}

impl RateLimiter {
    /// A limiter on `clock`, waiting through `sleeper`.
    pub fn new(clock: Arc<dyn Clock>, sleeper: Arc<dyn Sleeper>) -> Self {
        Self {
            clock,
            sleeper,
            arrivals: Mutex::new(BTreeMap::new()),
        }
    }

    /// Waits until `connection` may make one more call under `limit`.
    pub async fn acquire(&self, connection: ConnectionId, limit: RateLimit) {
        let due = self.reserve(connection, limit);
        if due > self.clock.now() {
            self.sleeper.sleep_until(due).await;
        }
    }

    /// Reserves the next slot and returns when it is due.
    fn reserve(&self, connection: ConnectionId, limit: RateLimit) -> DateTime<Utc> {
        let now = self.clock.now();
        let requests = limit.requests.max(1);
        let interval = TimeDelta::from_std(limit.per / requests).unwrap_or(TimeDelta::zero());
        let burst = interval * i32::try_from(requests - 1).unwrap_or(i32::MAX);
        let mut arrivals = self.arrivals.lock().unwrap_or_else(PoisonError::into_inner);
        let arrival = arrivals.get(&connection).copied().unwrap_or(now).max(now);
        arrivals.insert(connection, arrival + interval);
        (arrival - burst).max(now)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::TimeZone;
    use deino_domain::FakeClock;

    use super::*;
    use crate::FakeSleeper;

    fn limiter() -> (RateLimiter, Arc<FakeClock>, Arc<FakeSleeper>) {
        let clock = Arc::new(FakeClock::new(
            Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap(),
        ));
        let sleeper = Arc::new(FakeSleeper::new(clock.clone()));
        (
            RateLimiter::new(clock.clone(), sleeper.clone()),
            clock,
            sleeper,
        )
    }

    const TWO_PER_SECOND: RateLimit = RateLimit {
        requests: 2,
        per: Duration::from_secs(1),
    };

    #[tokio::test]
    async fn a_burst_passes_and_the_next_call_waits_for_its_slot() {
        let (limiter, clock, sleeper) = limiter();
        let start = clock.now();
        let id = ConnectionId::generate(clock.as_ref());

        limiter.acquire(id, TWO_PER_SECOND).await;
        limiter.acquire(id, TWO_PER_SECOND).await;
        assert!(sleeper.waits().is_empty());

        limiter.acquire(id, TWO_PER_SECOND).await;
        assert_eq!(sleeper.waits(), [start + TimeDelta::milliseconds(500)]);
    }

    #[tokio::test]
    async fn connections_are_paced_separately() {
        let (limiter, clock, sleeper) = limiter();
        let one = ConnectionId::generate(clock.as_ref());
        let two = ConnectionId::generate(clock.as_ref());
        let single = RateLimit {
            requests: 1,
            per: Duration::from_secs(1),
        };

        limiter.acquire(one, single).await;
        limiter.acquire(two, single).await;

        assert!(sleeper.waits().is_empty());
    }
}
