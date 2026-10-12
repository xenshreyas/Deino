//! [`StatsService`]: the Stats page's numbers, computed on read.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use deino_db::Database;
use deino_domain::{Clock, LocalZone};

use super::aggregate::{self, count};
use super::repo::{SqliteStatsRepo, StatsRepo};
use super::window;
use super::{Heatmap, StatsError, StatsOverview, StatsRange};
use crate::profile::InternalDomains;
use crate::settings::SettingsService;

/// What the stats and recap services read.
#[derive(Clone)]
pub struct StatsPorts {
    /// The rows stats are computed from.
    pub repo: Arc<dyn StatsRepo>,
    /// The profile, which says who the user is and what is internal.
    pub settings: Arc<SettingsService>,
    /// The user's time zone, for local days and hours.
    pub zone: Arc<dyn LocalZone>,
    /// Now.
    pub clock: Arc<dyn Clock>,
}

impl StatsPorts {
    /// The ports over the app database.
    pub fn sqlite(
        database: &Database,
        settings: Arc<SettingsService>,
        zone: Arc<dyn LocalZone>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo: Arc::new(SqliteStatsRepo::new(database.clone())),
            settings,
            zone,
            clock,
        }
    }
}

/// Meeting and usage stats.
pub struct StatsService {
    ports: StatsPorts,
}

impl StatsService {
    /// A service over `ports`.
    pub fn new(ports: StatsPorts) -> Self {
        Self { ports }
    }

    /// The numbers of `range`, ending now.
    pub async fn overview(&self, range: StatsRange) -> Result<StatsOverview, StatsError> {
        let since = window::since(range, self.ports.clock.now(), self.ports.zone.as_ref());
        self.since(since).await
    }

    /// The numbers of everything since `since` (all time when `None`).
    pub(super) async fn since(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<StatsOverview, StatsError> {
        let ports = &self.ports;
        let meetings = ports.repo.meetings(since).await?;
        let attendance = ports.repo.attendance(since).await?;
        let usage = ports.repo.llm_usage(since).await?;
        let domains = InternalDomains::of(&ports.settings.get().await.profile);
        let zone = ports.zone.as_ref();
        let (people, top_people) = aggregate::people(&attendance, &domains);
        Ok(StatsOverview {
            since,
            meetings: count(meetings.len()),
            recorded_seconds: aggregate::recorded_seconds(&meetings),
            people,
            audience: aggregate::audience(&meetings, &attendance, &domains),
            top_people,
            weekdays: aggregate::by_weekday(&meetings, zone),
            hours: aggregate::by_hour(&meetings, zone),
            llm: aggregate::llm_usage(usage),
        })
    }

    /// Meetings per day over the last year, whatever the page's period.
    pub async fn heatmap(&self) -> Result<Heatmap, StatsError> {
        let zone = self.ports.zone.as_ref();
        let (first_day, last_day) = window::heatmap_days(self.ports.clock.now(), zone);
        let meetings = self
            .ports
            .repo
            .meetings(Some(zone.start_of(first_day)))
            .await?;
        Ok(Heatmap {
            first_day,
            last_day,
            days: aggregate::per_day(&meetings, zone, (first_day, last_day)),
        })
    }
}
