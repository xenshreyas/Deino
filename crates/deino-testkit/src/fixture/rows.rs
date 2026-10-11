//! The [`SeedTarget`](super::SeedTarget)s: one per group of tables, run in
//! foreign-key order by [`FixtureSeeder`](super::FixtureSeeder).
//!
//! | Target | Tables |
//! |---|---|
//! | [`CalendarRows`] | `calendar_accounts`, `calendars`, `calendar_events` |
//! | [`MeetingRows`] | `meetings` (with its calendar link and series) |
//! | [`PeopleRows`] | `companies`, `people`, `meeting_attendees` |
//! | [`TranscriptRows`] | `speakers`, `transcript_segments` |
//! | [`NoteRows`] | `meeting_notes` (the user's own notes, NOT-01) |
//!
//! Every row gets a stable ID from [`fixture_uuid`](super::ids::fixture_uuid),
//! so re-seeding rewrites the same rows. The SQL is written here rather than
//! through `deino-core`'s repositories because this crate must not depend on
//! `deino-core` (see the crate README); core's tests read the seeded rows
//! back through those repositories, which keeps the two in step. Ground-truth
//! action items are eval answers, not data, so they are never seeded.

mod calendar;
mod meeting;
mod notes;
mod people;
mod transcript;

pub use calendar::CalendarRows;
pub use meeting::MeetingRows;
pub use notes::NoteRows;
pub use people::PeopleRows;
pub use transcript::TranscriptRows;
