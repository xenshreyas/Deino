//! [`ImportRunner::sqlite`]: the runner over the app's SQLite repositories,
//! shared by the composition root and the tests, so both write through the
//! same `TranscriptImporter` with people and action items.

use std::sync::Arc;

use deino_db::Database;
use deino_domain::Clock;

use super::{ImportRunner, SqliteImportLedger};
use crate::enhance::SqliteEnhancedNotesRepo;
use crate::entities::EntityPorts;
use crate::meetings::{MeetingWork, SqliteMeetingRepo};
use crate::segments::SqliteSegmentRepo;
use crate::tags::TagService;
use crate::transcript_import::{ImportPeoplePorts, TranscriptImportPorts, TranscriptImporter};

impl ImportRunner {
    /// A runner over `database`'s repositories, applying tags through
    /// `tags` and scheduling the search index through `work`.
    pub fn sqlite(
        database: &Database,
        tags: Arc<TagService>,
        work: Arc<dyn MeetingWork>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        let entities = EntityPorts::sqlite(database, Arc::clone(&clock));
        let importer = TranscriptImporter::new(TranscriptImportPorts {
            meetings: meetings.clone(),
            trash: meetings,
            speakers: segments.clone(),
            segments,
            notes: Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
            tags,
            work,
            clock: Arc::clone(&clock),
        })
        .with_people(ImportPeoplePorts {
            directory: entities.directory,
            attendees: entities.attendees,
            action_items: entities.action_items,
        });
        Self::new(
            Arc::new(importer),
            Arc::new(SqliteImportLedger::new(database.clone())),
            clock,
        )
    }
}
