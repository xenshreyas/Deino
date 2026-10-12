//! When to try again, when to give up and when to turn an endpoint off:
//! pure rules over attempt counts and times, driven by a fake clock in tests.
//!
//! - **Timeout:** each attempt may take [`ATTEMPT_TIMEOUT`] (15 s).
//! - **Retry:** a network error, a timeout, 408, 429 or 5xx is tried again
//!   after 30 s, doubling each time up to 2 h ([`retry_delay`]), until the
//!   delivery is [`RETRY_WINDOW`] (24 h) old. Retries happen while the app
//!   runs; pending deliveries are picked up again after a restart.
//! - **Give up at once:** any other status (3xx, 4xx) fails the delivery.
//! - **Auto-disable:** an endpoint whose attempts have all failed for 24 h
//!   (and at least [`DISABLE_MIN_FAILURES`] times) is turned off, and so is
//!   one that answers 410 Gone ([`disable_reason`]).

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};

use super::DisabledReason;

/// How long one attempt may take, end to end.
pub const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);
/// How long after the event Deino keeps retrying.
pub const RETRY_WINDOW: TimeDelta = TimeDelta::hours(24);
/// The first retry's delay; each later one doubles it.
pub const FIRST_RETRY: TimeDelta = TimeDelta::seconds(30);
/// The longest delay between two attempts.
pub const MAX_RETRY: TimeDelta = TimeDelta::hours(2);
/// Failed attempts in a row before a day of failures turns an endpoint off,
/// so a Mac that slept through the day does not disable it on one attempt.
pub const DISABLE_MIN_FAILURES: u32 = 5;

/// What one attempt came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttemptResult {
    /// The endpoint answered 2xx.
    Delivered(u16),
    /// The endpoint answered another status.
    Status(u16),
    /// No answer: connection, TLS or timeout; the user-safe reason.
    NoAnswer(String),
    /// Deino's network policy stopped it (offline mode, the service switched
    /// off): retried, but not the endpoint's fault, so it never counts
    /// towards turning the endpoint off.
    Blocked(String),
    /// Deino could not make the request at all (an address the network
    /// layer refuses): never retried.
    Rejected(String),
}

impl AttemptResult {
    /// True when trying again may succeed.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Delivered(_) | Self::Rejected(_) => false,
            Self::Status(status) => matches!(status, 408 | 429 | 500..=599),
            Self::NoAnswer(_) | Self::Blocked(_) => true,
        }
    }

    /// The HTTP status, if there was an answer.
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Delivered(status) | Self::Status(status) => Some(*status),
            Self::NoAnswer(_) | Self::Blocked(_) | Self::Rejected(_) => None,
        }
    }

    /// The user-safe error for the log, `None` on success.
    pub fn error(&self) -> Option<String> {
        match self {
            Self::Delivered(_) => None,
            Self::Status(status) => Some(format!("the endpoint answered {status}")),
            Self::NoAnswer(reason) | Self::Blocked(reason) | Self::Rejected(reason) => {
                Some(reason.clone())
            }
        }
    }
}

/// The delay before the attempt after `attempts` failed ones (1-based):
/// 30 s, 1 min, 2 min, … doubling, at most 2 h.
pub fn retry_delay(attempts: u32) -> TimeDelta {
    let doublings = attempts.saturating_sub(1).min(16);
    FIRST_RETRY
        .checked_mul(1 << doublings)
        .map_or(MAX_RETRY, |delay| delay.min(MAX_RETRY))
}

/// When to try a delivery made at `created_at` again after its
/// `attempts`-th failure at `now`; `None` to give up.
pub fn next_attempt(
    result: &AttemptResult,
    attempts: u32,
    (created_at, now): (DateTime<Utc>, DateTime<Utc>),
) -> Option<DateTime<Utc>> {
    if !result.is_retryable() {
        return None;
    }
    let next = now + retry_delay(attempts);
    (next < created_at + RETRY_WINDOW).then_some(next)
}

/// Why an endpoint should be turned off after a failed attempt at `now`,
/// given its failure streak (counting this attempt) and when it began.
pub fn disable_reason(
    result: &AttemptResult,
    streak: u32,
    (failing_since, now): (DateTime<Utc>, DateTime<Utc>),
) -> Option<DisabledReason> {
    if result.status() == Some(410) {
        return Some(DisabledReason::Gone);
    }
    let a_day = now - failing_since >= RETRY_WINDOW;
    (a_day && streak >= DISABLE_MIN_FAILURES).then_some(DisabledReason::Failing)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use deino_domain::{Clock, FakeClock};

    use super::*;

    fn start() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 12, 9, 0, 0).unwrap()
    }

    #[test]
    fn delays_double_from_thirty_seconds_up_to_two_hours() {
        let delays: Vec<i64> = (1..=11).map(|n| retry_delay(n).num_seconds()).collect();

        assert_eq!(
            delays,
            [30, 60, 120, 240, 480, 960, 1920, 3840, 7200, 7200, 7200]
        );
        assert_eq!(retry_delay(u32::MAX), MAX_RETRY);
    }

    #[test]
    fn a_failing_endpoint_is_retried_on_schedule_for_a_day_then_given_up() {
        let clock = FakeClock::new(start());
        let failure = AttemptResult::Status(503);
        let mut attempts = 0;
        let mut times = vec![];
        loop {
            attempts += 1;
            let Some(next) = next_attempt(&failure, attempts, (start(), clock.now())) else {
                break;
            };
            clock.advance(next - clock.now());
            times.push((clock.now() - start()).num_seconds());
        }

        assert_eq!(&times[..4], [30, 90, 210, 450]);
        assert!(clock.now() - start() < RETRY_WINDOW);
        assert!(clock.now() + MAX_RETRY >= start() + RETRY_WINDOW);
        assert_eq!(attempts, 19, "{times:?}");
    }

    #[test]
    fn only_timeouts_rate_limits_server_errors_and_no_answer_are_retried() {
        let now = (start(), start());
        for retryable in [408, 429, 500, 502, 503, 599] {
            assert!(next_attempt(&AttemptResult::Status(retryable), 1, now).is_some());
        }
        assert!(next_attempt(&AttemptResult::NoAnswer("timed out".into()), 1, now).is_some());
        assert!(next_attempt(&AttemptResult::Blocked("offline".into()), 1, now).is_some());
        for permanent in [301, 400, 401, 404, 410, 422] {
            assert_eq!(
                next_attempt(&AttemptResult::Status(permanent), 1, now),
                None
            );
        }
        assert_eq!(next_attempt(&AttemptResult::Delivered(204), 1, now), None);
    }

    #[test]
    fn an_endpoint_is_turned_off_after_a_day_of_failures_or_a_410() {
        let failing = AttemptResult::Status(500);
        let day = start() + RETRY_WINDOW;

        assert_eq!(
            disable_reason(&failing, 18, (start(), day - TimeDelta::seconds(1))),
            None
        );
        assert_eq!(disable_reason(&failing, 4, (start(), day)), None);
        assert_eq!(
            disable_reason(&failing, 5, (start(), day)),
            Some(DisabledReason::Failing)
        );
        assert_eq!(
            disable_reason(&AttemptResult::Status(410), 1, (start(), start())),
            Some(DisabledReason::Gone)
        );
    }
}
