//! Test fixtures and fakes for Deino's crates. A dev-dependency only.
//!
//! - [`test_db`] and [`FakeClock`] are re-exported from the crates that own
//!   them, so tests have one import path;
//! - [`at`] writes timestamps readably;
//! - [`seed`] inserts rows that no feature can create yet (the legacy
//!   recording path still creates meetings).
//!
//! This crate depends only on `deino-domain` and `deino-db`. It must never
//! depend on `deino-core`: core's own tests use it, and a dependency cycle
//! would compile two incompatible copies of core's types.

#![allow(
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "test-only crate: a broken fixture must fail the calling test at once"
)]

pub mod seed;

use chrono::{DateTime, Utc};
pub use deino_db::test_db;
pub use deino_domain::FakeClock;

/// Parses an RFC 3339 timestamp, for example `at("2026-10-08T09:00:00Z")`.
pub fn at(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .expect("tests pass valid RFC 3339 timestamps")
        .to_utc()
}
