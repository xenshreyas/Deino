//! Test harness: a migrated database, a profile, a pinned clock and zone.

#![allow(
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "test helpers: a broken seed must fail the calling test at once"
)]

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{Email, FixedZone};
use deino_testkit::{FakeClock, at};

use super::{StatsPorts, StatsService};
use crate::settings::{FakeSettingsStore, RecordedSettingsEvents, Settings, SettingsService};

/// "Now" in the stats tests: a Sunday, after every fixture meeting.
pub const NOW: &str = "2026-10-11T12:00:00Z";

/// A database, settings and a clock for stats tests.
pub struct World {
    /// The migrated in-memory database.
    pub database: Database,
    /// The clock, at [`NOW`] until advanced.
    pub clock: Arc<FakeClock>,
    settings: Arc<SettingsService>,
}

impl World {
    /// An empty world whose profile lists `emails` as the user's addresses.
    pub async fn new(emails: &[&str]) -> Self {
        let mut saved = Settings::default();
        saved.profile.emails = emails
            .iter()
            .map(|email| Email::parse(email).expect("a valid address"))
            .collect();
        let store = Arc::new(FakeSettingsStore::with(Some(saved)));
        let settings = SettingsService::load(store, Arc::new(RecordedSettingsEvents::default()))
            .await
            .expect("the settings load");
        Self {
            database: test_db().await,
            clock: Arc::new(FakeClock::new(at(NOW))),
            settings: Arc::new(settings),
        }
    }

    /// The stats ports over this world, in UTC.
    pub fn ports(&self) -> StatsPorts {
        StatsPorts::sqlite(
            &self.database,
            Arc::clone(&self.settings),
            Arc::new(FixedZone::utc()),
            self.clock.clone(),
        )
    }

    /// The stats service over this world.
    pub fn stats(&self) -> StatsService {
        StatsService::new(self.ports())
    }

    /// Runs `sql` on the writer.
    pub async fn run(&self, sql: &str) {
        sqlx::query(sql)
            .execute(self.database.writer())
            .await
            .expect("the seed SQL is valid");
    }
}
