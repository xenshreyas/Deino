//! [`ReadPorts`]: the repositories and services the library reads through,
//! and their SQLite wiring.

use std::sync::Arc;

use deino_db::Database;
use deino_domain::{Clock, LocalZone};

use super::{HybridSearch, Search};
use crate::action_items::{
    ActionItemDeps, ActionItemService, NoActionItemEvents, OptionSources, SqliteActionItemRepo,
};
use crate::calendar::{CalendarEventRepo, SqliteCalendarRepo};
use crate::companies::{CompanyRepo, SqliteCompanyRepo};
use crate::enhance::{EnhancedNotesReader, SqliteEnhancedNotesRepo};
use crate::export::ExportPorts;
use crate::folders::{FolderPorts, FolderRepo, FolderService, SqliteFolderRepo};
use crate::people::SqlitePeopleRepo;
use crate::search::semantic::Embedder;
use crate::tags::{MeetingTagRepo, SqliteTagRepo, TagRepo};

/// Everything the library reads through.
#[derive(Clone)]
pub struct ReadPorts {
    /// What a meeting is made of: the same repositories exports read, so a
    /// meeting reads the same here as in an export.
    pub meeting: ExportPorts,
    /// The AI notes' versions, for their citations.
    pub enhanced: Arc<dyn EnhancedNotesReader>,
    /// Every tag.
    pub tags: Arc<dyn TagRepo>,
    /// Which meetings carry which tag.
    pub tagging: Arc<dyn MeetingTagRepo>,
    /// Every folder.
    pub folders: Arc<dyn FolderRepo>,
    /// The folder tree with counts.
    pub folder_tree: Arc<FolderService>,
    /// Every company.
    pub companies: Arc<dyn CompanyRepo>,
    /// Synced calendar events.
    pub events: Arc<dyn CalendarEventRepo>,
    /// Action items and their filters.
    pub action_items: Arc<ActionItemService>,
    /// Text search.
    pub search: Arc<dyn Search>,
    /// The time, for "today" and default windows.
    pub clock: Arc<dyn Clock>,
}

/// What the library needs besides the database.
#[derive(Clone)]
pub struct ReadContext {
    /// The user's time zone.
    pub zone: Arc<dyn LocalZone>,
    /// The time.
    pub clock: Arc<dyn Clock>,
    /// The local embedding model, for semantic search; `None` searches by
    /// keyword only.
    pub embedder: Option<Arc<dyn Embedder>>,
}

impl ReadPorts {
    /// Every port over the SQLite repositories of `database`.
    pub fn sqlite(database: &Database, context: &ReadContext) -> Self {
        let ReadContext {
            zone,
            clock,
            embedder,
        } = context.clone();
        let tags = Arc::new(SqliteTagRepo::new(database.clone()));
        let folders = Arc::new(SqliteFolderRepo::new(database.clone()));
        let companies = Arc::new(SqliteCompanyRepo::new(database.clone()));
        Self {
            meeting: ExportPorts::sqlite(database, zone.clone()),
            enhanced: Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
            tags: tags.clone(),
            tagging: tags.clone(),
            folders: folders.clone(),
            folder_tree: Arc::new(FolderService::new(&FolderPorts::over(
                folders.clone(),
                clock.clone(),
            ))),
            companies: companies.clone(),
            events: Arc::new(SqliteCalendarRepo::new(database.clone())),
            search: Arc::new(HybridSearch::new(
                database.reader().clone(),
                embedder,
                zone.clone(),
            )),
            action_items: Arc::new(action_items(
                database,
                &Shared {
                    tags,
                    folders,
                    companies,
                },
                zone,
                clock.clone(),
            )),
            clock,
        }
    }
}

/// Repositories the action items' filter options share with the library.
struct Shared {
    tags: Arc<SqliteTagRepo>,
    folders: Arc<SqliteFolderRepo>,
    companies: Arc<SqliteCompanyRepo>,
}

/// The action items service; its writes are never called here.
fn action_items(
    database: &Database,
    shared: &Shared,
    zone: Arc<dyn LocalZone>,
    clock: Arc<dyn Clock>,
) -> ActionItemService {
    let repo = Arc::new(SqliteActionItemRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    ActionItemService::new(ActionItemDeps {
        items: repo.clone(),
        batch: repo.clone(),
        reader: repo,
        me: people.clone(),
        options: OptionSources {
            people,
            companies: shared.companies.clone(),
            tags: shared.tags.clone(),
            folders: shared.folders.clone(),
        },
        events: Arc::new(NoActionItemEvents),
        clock,
        zone,
    })
}
