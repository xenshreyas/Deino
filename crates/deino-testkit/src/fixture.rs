//! Fixture meetings: the repository's `fixtures/` loaded into typed values,
//! and seeded into a database.
//!
//! - [`Fixtures`] reads `fixtures/meetings/*.json`. Parsing is the schema
//!   check: unknown fields, bad enums and malformed emails fail to
//!   deserialize, and `validate` rejects incoherent meetings (unknown
//!   speakers, segments out of order, evidence that is not in the transcript).
//! - [`FixtureSeeder`] writes them through [`SeedTarget`]s, one per group of
//!   tables, in one transaction; [`seed_fixtures`] is the one-call form.
//! - [`PcmTrack`] decodes the fixture audio of the two meetings that have it.
//!
//! The same JSON feeds the frontend's mocked IPC (`pnpm dev:mock`), which
//! validates it with zod, and the AI evals, which read `groundTruth`.

mod audio;
mod legacy_rows;
mod load;
mod model;
mod seeding;
mod validate;
mod vocab;

pub use audio::{AudioError, PcmTrack};
pub use legacy_rows::{LegacyMeetingRows, LegacyTranscriptRows};
pub use load::{FixtureError, Fixtures};
pub use model::{
    Attendee, CalendarEvent, Company, ExpectedActionItem, ExpectedFact, FixtureAudio,
    FixtureMeeting, GroundTruth, Segment, Speaker, USER_EMAIL, Word,
};
pub use seeding::{FixtureSeeder, SeedError, SeedReport, SeedTarget, seed_fixtures};
pub use validate::{ID_PREFIX, Problem, validate};
pub use vocab::{
    AttendeeRole, Channel, Email, InvalidEmail, MeetingKind, Millis, Platform, Recurrence, Rsvp,
};

#[cfg(test)]
mod fixture_tests;
