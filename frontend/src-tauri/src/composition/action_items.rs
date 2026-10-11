//! Wiring for action items (AI-05): the items and saved-view services the
//! `action-items` plugin calls, the extraction behind the pipeline's
//! `extract_action_items` stage (the gateway's `extract.action_items` route)
//! and the morning reminder, all over the SQLite repositories, the system
//! clock and the Mac's time zone.

use std::sync::Arc;

use deino_core::action_items::extraction::{
    ActionItemExtractor, ActionItemModel, ContextSources, ExtractionDeps, ExtractionService,
    GatewayActionItemModel, NoActionItemModel,
};
use deino_core::action_items::{
    ActionItemDeps, ActionItemEvents, ActionItemService, DailyReminder, OptionSources,
    ReminderDeps, SavedViewService, SqliteActionItemRepo,
};
use deino_core::companies::SqliteCompanyRepo;
use deino_core::folders::SqliteFolderRepo;
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::notes::SqliteNotesRepo;
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::SettingsService;
use deino_core::tags::SqliteTagRepo;
use deino_db::Database;
use deino_domain::{Clock, LocalZone, SystemZone};
use deino_llm::Gateway;
use tauri::{AppHandle, Manager};

use crate::features::action_items::{SystemNotifier, TauriActionItemEvents, start_reminders};

/// What action items need besides the database.
pub(super) struct ActionItemWiring {
    /// The gateway, when it could be wired; without it extraction fails as
    /// unavailable and the stage waits.
    pub(super) gateway: Option<Arc<Gateway>>,
    /// The reminder's switch.
    pub(super) settings: Arc<SettingsService>,
    /// The time.
    pub(super) clock: Arc<dyn Clock>,
}

/// The adapters every action item service shares.
struct Shared {
    repo: Arc<SqliteActionItemRepo>,
    events: Arc<dyn ActionItemEvents>,
    zone: Arc<dyn LocalZone>,
}

/// Builds and manages the items and saved-view services, starts the morning
/// reminder, and returns the extraction for the pipeline.
pub(super) fn attach_action_items(
    app: &AppHandle,
    database: &Database,
    wiring: &ActionItemWiring,
) -> Arc<dyn ActionItemExtractor> {
    let shared = Shared {
        repo: Arc::new(SqliteActionItemRepo::new(database.clone())),
        events: Arc::new(TauriActionItemEvents::new(app.clone())),
        zone: Arc::new(SystemZone),
    };
    let items = Arc::new(items_service(database, &shared, wiring));
    start_reminders(Arc::new(DailyReminder::new(ReminderDeps {
        items: items.clone(),
        switch: wiring.settings.clone(),
        notifier: Arc::new(SystemNotifier::new(app.clone())),
        clock: wiring.clock.clone(),
        zone: shared.zone.clone(),
    })));
    app.manage(items);
    let views = SavedViewService::new(shared.repo.clone(), wiring.clock.clone());
    app.manage(Arc::new(views));
    extraction(database, shared, wiring)
}

/// The items service over the SQLite repositories.
fn items_service(
    database: &Database,
    shared: &Shared,
    wiring: &ActionItemWiring,
) -> ActionItemService {
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    ActionItemService::new(ActionItemDeps {
        items: shared.repo.clone(),
        batch: shared.repo.clone(),
        reader: shared.repo.clone(),
        me: people.clone(),
        options: OptionSources {
            people,
            companies: Arc::new(SqliteCompanyRepo::new(database.clone())),
            tags: Arc::new(SqliteTagRepo::new(database.clone())),
            folders: Arc::new(SqliteFolderRepo::new(database.clone())),
        },
        events: shared.events.clone(),
        clock: wiring.clock.clone(),
        zone: shared.zone.clone(),
    })
}

/// The extraction service over the meeting's repositories and the model.
fn extraction(
    database: &Database,
    shared: Shared,
    wiring: &ActionItemWiring,
) -> Arc<dyn ActionItemExtractor> {
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let model: Arc<dyn ActionItemModel> = match &wiring.gateway {
        Some(gateway) => Arc::new(GatewayActionItemModel::new(gateway.clone())),
        None => Arc::new(NoActionItemModel),
    };
    Arc::new(ExtractionService::new(ExtractionDeps {
        sources: ContextSources {
            meetings: meetings.clone(),
            lifecycle: meetings,
            segments: segments.clone(),
            speakers: segments,
            attendees: people.clone(),
            people: people.clone(),
            me: people,
            notes: Arc::new(SqliteNotesRepo::new(database.clone())),
        },
        model,
        items: shared.repo.clone(),
        batch: shared.repo,
        events: shared.events,
        clock: wiring.clock.clone(),
        zone: shared.zone,
    }))
}
