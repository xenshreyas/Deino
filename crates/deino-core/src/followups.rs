//! Suggested follow-up emails (NOT-07): after a meeting with at least one
//! external attendee and at most ten people, the model first decides
//! whether the user should follow up by email, and only then drafts a
//! subject and body with `[placeholders]` for what only the user knows.
//! Home lists ready drafts for three days; the meeting shows a "View draft"
//! banner. Sending is the user's mail app (`mailto:`) or the clipboard;
//! sending through Gmail or Outlook waits for INT-07 (§8 D12).
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`FollowUpDraft`], [`Recipients`], [`FollowUpEdit`], the stored row |
//! | `audience.rs` | The pure rules: who is external, the ten-person cap ([`judge`]) |
//! | `inputs.rs` | [`FollowUpInputs`]: the meeting, its people, notes and transcript, redacted (NOT-09), and the two prompts |
//! | `writer.rs` | [`FollowUpModel`] and its gateway adapter (route `followup.email`, structured output) |
//! | `repo.rs`, `participants.rs` | [`FollowUpRepo`] over `follow_up_drafts`; [`ParticipantReader`] over ORG-04's people and companies |
//! | `generate.rs` | [`FollowUpGenerator`]: the `follow_up_draft` stage's rules ([`FollowUpDrafter`]) |
//! | `service.rs` | [`FollowUpService`]: read, edit, dismiss (permanent), open in Mail, copy |

mod audience;
mod error;
mod generate;
mod inputs;
mod model;
mod participants;
mod repo;
mod service;
mod writer;

#[cfg(test)]
mod acceptance_tests;
#[cfg(test)]
mod repo_tests;
#[cfg(test)]
mod testing;

pub use audience::{Ineligible, MAX_PEOPLE, Participant, judge};
pub use error::FollowUpError;
pub use generate::{FollowUpDeps, FollowUpDrafter, FollowUpGenerator, FollowUpRun, Kept};
pub use inputs::{FollowUpInputs, MAX_NOTES_CHARS, MAX_TRANSCRIPT_CHARS};
pub use model::{
    FollowUpDraft, FollowUpEdit, FollowUpStatus, FollowUpSummary, QuietReason, Recipients,
    StoredFollowUp,
};
pub use participants::ParticipantReader;
pub use repo::{FollowUpRepo, SqliteFollowUpRepo};
pub use service::{FollowUpPorts, FollowUpService, SHOWN_ON_HOME};
pub use writer::{FollowUpModel, GatewayFollowUpModel};
