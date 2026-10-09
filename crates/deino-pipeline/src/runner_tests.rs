//! The runner end to end on a real database, with fake stages: order,
//! retries and backoff, failures and re-runs, cancellation, the concurrency
//! limits, progress events, and resuming after a restart.

mod harness;
mod lifecycle_tests;
mod restart_tests;
mod scheduling_tests;
