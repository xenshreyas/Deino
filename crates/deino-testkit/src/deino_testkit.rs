//! Test fixtures and fakes for Deino's crates.
//!
//! - [`fixture`]: the fixture meetings in `fixtures/`, as typed values, a
//!   seeder that writes them into a database, and their decoded audio;
//! - [`test_db`] and [`FakeClock`] (feature `test-support`) are re-exported
//!   from the crates that own them, so tests have one import path;
//! - [`at`] writes timestamps readably;
//! - [`seed`] inserts single rows that no feature can create yet.
//!
//! Tests use this crate as a dev-dependency with `test-support`. The app's
//! debug-only `dev` plugin also uses [`fixture`] (to seed a real database)
//! without that feature, so release builds never compile fakes in.
//!
//! This crate depends only on `deino-domain` and `deino-db`. It must never
//! depend on `deino-core`: core's own tests use it, and a dependency cycle
//! would compile two incompatible copies of core's types.

pub mod fixture;
pub mod seed;

use chrono::{DateTime, Utc};
#[cfg(feature = "test-support")]
pub use deino_db::test_db;
#[cfg(feature = "test-support")]
pub use deino_domain::FakeClock;
pub use fixture::{FixtureSeeder, Fixtures, seed_fixtures};

/// Parses an RFC 3339 timestamp, for example `at("2026-10-08T09:00:00Z")`.
///
/// # Panics
///
/// When `rfc3339` is not a valid timestamp, which fails the calling test.
#[allow(
    clippy::expect_used,
    reason = "test helper: a malformed literal must fail the test at once"
)]
pub fn at(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .expect("tests pass valid RFC 3339 timestamps")
        .to_utc()
}
