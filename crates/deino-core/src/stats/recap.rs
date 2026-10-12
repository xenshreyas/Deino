//! [`RecapService`]: the year in review, generated once and kept.
//!
//! The recap opens in December (Granola's "Crunched" came out in early
//! December, `docs/research/granola.md` §3.7); [`RecapWindow::Anytime`] is
//! the debug flag that opens it all year for testing. It is written on the
//! gateway's `recap` route from the year's aggregated numbers and meeting
//! titles only, and stored in `year_recaps`, so it costs one model call.

use std::sync::Arc;

use chrono::{DateTime, Datelike, Utc};
use deino_llm::{Gateway, Route};

use super::recap_prompt::{TITLES_READ, recap_prompt};
use super::recap_repo::RecapRepo;
use super::window::year_bounds;
use super::{RecapStatus, StatsError, StatsPorts, StatsService, YearRecap};
use crate::entities::visible_answer;

/// The month the recap opens.
const DECEMBER: u32 = 12;

/// When the year in review can be generated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecapWindow {
    /// In December only.
    #[default]
    December,
    /// Any time: the debug flag, for testing.
    Anytime,
}

/// What the recap service needs besides the stats ports.
pub struct RecapParts {
    /// Where recaps are kept.
    pub recaps: Arc<dyn RecapRepo>,
    /// The LLM gateway; `None` when no model is set up.
    pub gateway: Option<Arc<Gateway>>,
    /// When the recap opens.
    pub window: RecapWindow,
}

/// Generates and keeps the year in review.
pub struct RecapService {
    stats: StatsService,
    ports: StatsPorts,
    parts: RecapParts,
}

impl RecapService {
    /// A service over the stats `ports` and `parts`.
    pub fn new(ports: StatsPorts, recap: RecapParts) -> Self {
        Self {
            stats: StatsService::new(ports.clone()),
            ports,
            parts: recap,
        }
    }

    /// This year's recap, if generated, and whether it can be.
    pub async fn status(&self) -> Result<RecapStatus, StatsError> {
        let now = self.ports.clock.now();
        let year = self.local_year(now);
        Ok(RecapStatus {
            year,
            available: self.is_open(now),
            recap: self.parts.recaps.get(year).await?,
        })
    }

    /// This year's recap: the stored one, or a new one written now.
    pub async fn generate(&self) -> Result<YearRecap, StatsError> {
        let now = self.ports.clock.now();
        if !self.is_open(now) {
            return Err(StatsError::RecapNotYet);
        }
        let year = self.local_year(now);
        if let Some(recap) = self.parts.recaps.get(year).await? {
            return Ok(recap);
        }
        let markdown = self.write(year).await?;
        let recap = YearRecap {
            year,
            markdown,
            generated_at: now,
        };
        log::info!(target: "stats", "year in review generated for {year}");
        Ok(self.parts.recaps.save(&recap).await?)
    }

    /// Asks the model for `year`'s recap from its numbers and titles.
    async fn write(&self, year: i32) -> Result<String, StatsError> {
        let Some(gateway) = &self.parts.gateway else {
            return Err(StatsError::NoModel);
        };
        let zone = self.ports.zone.as_ref();
        let bounds = year_bounds(year, zone);
        let stats = self.stats.since(Some(bounds.0)).await?;
        let titles = self.ports.repo.titles(bounds, TITLES_READ).await?;
        let prompt = recap_prompt(year, &stats, &titles, zone);
        let completion = gateway.complete(Route::Recap, prompt).await?;
        visible_answer(&completion.text).ok_or(StatsError::EmptyAnswer)
    }

    fn local_year(&self, now: DateTime<Utc>) -> i32 {
        self.ports.zone.local(now).year()
    }

    fn is_open(&self, now: DateTime<Utc>) -> bool {
        self.parts.window == RecapWindow::Anytime || self.ports.zone.local(now).month() == DECEMBER
    }
}
