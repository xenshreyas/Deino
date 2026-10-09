//! Writing fixture meetings into a Deino database.
//!
//! A [`SeedTarget`] owns one group of tables. [`FixtureSeeder`] runs every
//! target for every meeting inside one transaction on the writer, so a seed
//! is all-or-nothing and the UI hears a single `DbChanged`. Seeding twice
//! restores the fixtures to their original state (a renamed fixture gets its
//! title back) and leaves the user's own meetings alone.
//!
//! The targets fill the core schema (FND-07), one group of tables each; see
//! [`rows`](super::rows). A card that adds a table the fixtures describe adds
//! a target to [`FixtureSeeder::targets`]; nothing else changes.

use async_trait::async_trait;
use deino_db::{Database, DbError};
use deino_domain::{ErrorClass, ErrorCode};
use serde::Serialize;
use sqlx::SqliteConnection;

use super::load::{FixtureError, Fixtures};
use super::model::FixtureMeeting;

/// Writes the rows of one group of tables for a fixture meeting.
#[async_trait]
pub trait SeedTarget: Send + Sync {
    /// Writes `meeting`'s rows on `connection`, inside the caller's
    /// transaction, replacing whatever an earlier seed wrote.
    async fn seed(
        &self,
        connection: &mut SqliteConnection,
        meeting: &FixtureMeeting,
    ) -> Result<(), DbError>;
}

/// What a seed wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SeedReport {
    /// Fixture meetings written.
    pub meetings: u32,
    /// Transcript segments written.
    pub segments: u32,
}

/// Why seeding failed.
#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    /// The fixture files could not be loaded.
    #[error("could not load the fixtures")]
    Load(#[from] FixtureError),
    /// The database rejected a write; nothing was written.
    #[error("could not write the fixtures")]
    Write(#[from] DbError),
}

impl ErrorClass for SeedError {
    fn code(&self) -> ErrorCode {
        ErrorCode::Internal
    }
}

impl From<sqlx::Error> for SeedError {
    fn from(error: sqlx::Error) -> Self {
        Self::Write(error.into())
    }
}

/// Seeds the repository's fixtures into a database, for tests and the
/// debug-only `dev` plugin.
pub struct FixtureSeeder {
    database: Database,
    targets: Vec<Box<dyn SeedTarget>>,
}

impl FixtureSeeder {
    /// A seeder over `database` with every table group that exists today.
    pub fn new(database: Database) -> Self {
        Self {
            database,
            targets: Self::targets(),
        }
    }

    /// The table groups a seed fills, in foreign-key order.
    pub fn targets() -> Vec<Box<dyn SeedTarget>> {
        vec![
            Box::new(super::rows::CalendarRows),
            Box::new(super::rows::MeetingRows),
            Box::new(super::rows::PeopleRows),
            Box::new(super::rows::TranscriptRows),
            Box::new(super::rows::NoteRows),
        ]
    }

    /// Loads the repository's fixtures and writes them.
    pub async fn seed(&self) -> Result<SeedReport, SeedError> {
        let fixtures = Fixtures::load()?;
        self.seed_meetings(fixtures.meetings()).await
    }

    /// Writes `meetings` in one transaction.
    pub async fn seed_meetings(
        &self,
        meetings: &[FixtureMeeting],
    ) -> Result<SeedReport, SeedError> {
        let mut transaction = self.database.writer().begin().await?;
        for meeting in meetings {
            for target in &self.targets {
                target.seed(&mut transaction, meeting).await?;
            }
        }
        transaction.commit().await?;
        let segments = meetings.iter().map(|meeting| meeting.segments.len()).sum();
        Ok(SeedReport {
            meetings: saturating_u32(meetings.len()),
            segments: saturating_u32(segments),
        })
    }
}

/// Seeds the repository's fixtures into `database`; see [`FixtureSeeder`].
pub async fn seed_fixtures(database: &Database) -> Result<SeedReport, SeedError> {
    FixtureSeeder::new(database.clone()).seed().await
}

fn saturating_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}
