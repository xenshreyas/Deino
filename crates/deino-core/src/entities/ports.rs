//! The ports the entity services are built from.

use std::sync::Arc;

use deino_db::Database;
use deino_domain::Clock;

use super::{EntityDirectory, EntityMerger, EntityReads, EntityWrites, SqliteEntityRepo};
use crate::action_items::{ActionItemRepo, SqliteActionItemRepo};
use crate::calendar::{CalendarEventRepo, SqliteCalendarRepo};
use crate::companies::{CompanyRepo, SqliteCompanyRepo};
use crate::people::{AttendeeRepo, PersonRepo, SqlitePeopleRepo};

/// Everything [`PersonService`](super::PersonService),
/// [`CompanyService`](super::CompanyService) and the summariser read and
/// write. Cloning shares the same adapters.
#[derive(Clone)]
pub struct EntityPorts {
    /// People, by ID, address or name.
    pub people: Arc<dyn PersonRepo>,
    /// Companies, by ID or domain.
    pub companies: Arc<dyn CompanyRepo>,
    /// Who attended which meeting.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// The lists, meetings and digests.
    pub reads: Arc<dyn EntityReads>,
    /// Summaries, notes and logos.
    pub writes: Arc<dyn EntityWrites>,
    /// Merges.
    pub merger: Arc<dyn EntityMerger>,
    /// Open action items (AI-05's repository; only `list_for_assignee`).
    pub action_items: Arc<dyn ActionItemRepo>,
    /// Upcoming calendar events.
    pub events: Arc<dyn CalendarEventRepo>,
    /// The upsert rules.
    pub directory: Arc<EntityDirectory>,
    /// Timestamps and IDs.
    pub clock: Arc<dyn Clock>,
}

impl EntityPorts {
    /// Every port over the SQLite repositories of `database`.
    pub fn sqlite(database: &Database, clock: Arc<dyn Clock>) -> Self {
        let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
        let companies = Arc::new(SqliteCompanyRepo::new(database.clone()));
        let entities = Arc::new(SqliteEntityRepo::new(database.clone()));
        let directory = Arc::new(EntityDirectory::new(
            people.clone(),
            companies.clone(),
            Arc::clone(&clock),
        ));
        Self {
            people: people.clone(),
            companies,
            attendees: people,
            reads: entities.clone(),
            writes: entities.clone(),
            merger: entities,
            action_items: Arc::new(SqliteActionItemRepo::new(database.clone())),
            events: Arc::new(SqliteCalendarRepo::new(database.clone())),
            directory,
            clock,
        }
    }
}
