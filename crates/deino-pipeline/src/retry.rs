//! How often and how soon a failing stage is tried again.

use chrono::TimeDelta;

/// Retry limits and exponential backoff for retryable stage errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Attempts per run, the first included.
    pub max_attempts: u32,
    /// The wait before the second attempt; each later wait doubles.
    pub first_delay: TimeDelta,
    /// The longest wait between two attempts.
    pub max_delay: TimeDelta,
}

impl RetryPolicy {
    /// Three attempts, 10 s then 20 s apart: long enough for a rate limit
    /// or a flaky network to pass, short enough that notes do not stall.
    pub const DEFAULT: Self = Self {
        max_attempts: 3,
        first_delay: TimeDelta::seconds(10),
        max_delay: TimeDelta::minutes(5),
    };

    /// True when a stage whose attempt `attempt` (from 1) failed may try again.
    pub fn allows_retry_after(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }

    /// How long to wait after attempt `attempt` (from 1) failed.
    pub fn delay_after(&self, attempt: u32) -> TimeDelta {
        let doublings = attempt.saturating_sub(1).min(u32::BITS - 2);
        let factor = 1_i32 << doublings;
        self.first_delay
            .checked_mul(factor)
            .map_or(self.max_delay, |delay| delay.min(self.max_delay))
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_policy_allows_three_attempts() {
        let policy = RetryPolicy::DEFAULT;

        assert!(policy.allows_retry_after(1));
        assert!(policy.allows_retry_after(2));
        assert!(!policy.allows_retry_after(3));
    }

    #[test]
    fn delays_double_from_the_first_and_stop_at_the_cap() {
        let policy = RetryPolicy::DEFAULT;

        assert_eq!(policy.delay_after(1), TimeDelta::seconds(10));
        assert_eq!(policy.delay_after(2), TimeDelta::seconds(20));
        assert_eq!(policy.delay_after(3), TimeDelta::seconds(40));
        assert_eq!(policy.delay_after(9), TimeDelta::minutes(5));
        assert_eq!(policy.delay_after(u32::MAX), TimeDelta::minutes(5));
    }
}
