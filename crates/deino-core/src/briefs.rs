//! Pre-meeting briefs (NOT-08, ADR 0047): two to four cited bullets that
//! prepare the user for an upcoming meeting, written from their own data
//! only (§8 D12, D13: no web, no email).
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`Brief`], [`BriefBullet`], [`BriefCitation`], the stored row |
//! | `schedule.rs` | The pure rules: which events qualify, when a run is due (06:00, an hour before) |
//! | `sources.rs`, `collect.rs` | [`BriefSources`]: earlier meetings, the series' last meeting, open action items, person and company summaries and the invitation, redacted when the setting is on (NOT-09) |
//! | `inputs.rs` | [`BriefInputs`]: the local "enough to say?" check, the numbered prompt, its fingerprint and the answer's validation |
//! | `writer.rs` | [`BriefWriter`] and its gateway adapter (route `brief`, structured output) |
//! | `repo.rs` | [`BriefRepo`] over the `briefs` table |
//! | `service.rs` | [`BriefService`]: `prepare` (only when the inputs changed) and the read API, `for_event` |
//! | `scheduler.rs` | [`BriefScheduler`]: the background job the app ticks every minute |

mod collect;
mod error;
mod inputs;
mod model;
mod repo;
mod schedule;
mod scheduler;
mod service;
mod sources;
mod writer;

#[cfg(test)]
mod acceptance_tests;
#[cfg(test)]
mod testing;

pub use error::BriefError;
pub use inputs::{
    BriefInputs, MAX_BULLETS, MAX_CITATIONS, MIN_BULLETS, Rendered, Source, SourceIndex,
};
pub use model::{
    Brief, BriefBullet, BriefCitation, BriefFeedback, BriefRun, BriefStatus, StoredBrief,
};
pub use repo::{BriefRepo, SqliteBriefRepo};
pub use schedule::{
    BriefTrigger, LEAD_TIME, MIN_GROUP_ATTENDEES, MORNING_RUN, due_trigger, due_window, qualifies,
};
pub use scheduler::{BriefScheduler, BriefSwitch, SchedulerDeps, UpcomingEvents};
pub use service::{BriefDeps, BriefService};
pub use sources::BriefSources;
pub use writer::{BriefWriter, GatewayBriefWriter};
