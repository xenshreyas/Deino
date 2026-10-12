//! Stats and the year in review (ORG-09).
//!
//! The Stats page reads how many meetings the user had, how long they
//! recorded, whom they met (internal or external, by
//! [`InternalDomains`](crate::profile::InternalDomains)), when meetings
//! happen, and what the LLM gateway's calls cost. Everything is computed on
//! read (PARITY_PLAN §4.0): the `stats_meetings` view is the one definition
//! of a counted meeting (outside the trash, not merely scheduled, not the
//! sample), and nothing is cached except the generated year in review.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`StatsRange`], [`StatsOverview`], [`Heatmap`], [`YearRecap`] |
//! | `repo.rs` | [`StatsRepo`]: the rows, through `queries/*.sql` |
//! | `window.rs`, `aggregate.rs`, `pricing.rs` | Pure: periods, buckets, estimated cost |
//! | `service.rs` | [`StatsService`] |
//! | `recap.rs`, `recap_prompt.rs`, `recap_repo.rs` | [`RecapService`] on the `recap` route |

mod aggregate;
mod error;
mod model;
pub mod pricing;
mod recap;
mod recap_prompt;
mod recap_repo;
mod repo;
mod service;
mod window;

#[cfg(test)]
mod recap_tests;
#[cfg(test)]
mod stats_bench;
#[cfg(test)]
mod stats_tests;
#[cfg(test)]
mod testing;

pub use aggregate::TOP_PEOPLE;
pub use error::StatsError;
pub use model::{
    AudienceSplit, Heatmap, HeatmapDay, LlmUsage, ModelUsage, RecapStatus, StatsOverview,
    StatsRange, TopPerson, YearRecap,
};
pub use recap::{RecapParts, RecapService, RecapWindow};
pub use recap_prompt::recap_prompt;
pub use recap_repo::{RecapRepo, SqliteRecapRepo};
pub use repo::{AttendanceFact, MeetingFact, SqliteStatsRepo, StatsRepo, TitleFact, UsageFact};
pub use service::{StatsPorts, StatsService};
