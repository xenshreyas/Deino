//! The composition root: the one place that turns adapters into services.
//!
//! Services in `deino-core` depend on ports (traits). This module picks the
//! implementations (SQLite repositories, the system clock, Tauri event
//! adapters) and registers each service as Tauri state, so a command receives
//! `State<'_, Arc<Service>>` and never builds anything itself (§3A dependency
//! inversion). Tests build the same services over fakes instead.
//!
//! Startup wires three groups, in order:
//!
//! 1. [`attach_settings`]: settings and Keychain secrets, which need no
//!    database and which everything else reads;
//! 2. [`attach_network`]: the one network client (`deino_net::Net`), whose
//!    policy reads the settings (FND-12);
//! 3. [`attach_database`], once `deino.sqlite` is open (legacy settings
//!    imported by [`legacy_settings_import`]): the shared services later
//!    steps are built from (folders and tags, the LLM gateway and search, the
//!    stage services, templates and the enhanced notes, the pipeline).
//!
//! **A leaf feature wires itself**: its `composition/<feature>.rs` registers
//! an attach hook ([`attach`]) that runs at its [`attach::Phase`], so it
//! needs no line here beyond its `mod` (alphabetical, one per line).

mod action_items;
mod api;
mod ask;
pub(crate) mod attach;
mod automations;
mod briefs;
pub(crate) mod browser;
mod calendar;
mod cloud_transcription;
mod consent;
mod detection;
mod developer;
mod diarize;
mod enhance;
mod entities;
mod example;
mod export;
mod follow;
mod followups;
mod hotkeys;
pub(crate) mod import;
mod importers;
mod insights;
mod integrations;
mod links;
mod llm;
mod meetings;
mod network;
mod notepad;
mod notes;
mod notes_edit;
pub(crate) mod onboarding;
mod organize;
mod pipeline;
mod player;
pub(crate) mod privacy;
mod recording;
mod retention;
mod screen;
mod search;
mod services;
mod settings;
mod speaker_tags;
mod speakers;
mod stats;
mod stt;
mod transcript;
mod vocabulary;
mod webhooks;

use std::sync::Arc;

use action_items::ActionItemWiring;
use attach::Context;
pub(crate) use calendar::CalendarSchedule;
use deino_core::search::SearchIndexing;
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::{Clock, SystemClock};
pub(crate) use detection::attach_detection;
pub(crate) use developer::attach_developer;
use enhance::NotesAi;
use export::ExportServices;
pub(crate) use meetings::TrashPurge;
/// The root HTTP client, for tests that talk to a loopback server.
#[cfg(test)]
pub(crate) use network::root_client;
pub(crate) use network::{attach_network, model_downloads};
use notes::{NoteDeps, NoteServices};
pub(crate) use onboarding::attach_permissions;
pub(crate) use organize::Organize;
#[cfg(test)]
use pipeline::{Pipeline, PipelineAdapters};
use pipeline::{StageAdapters, StageServices, attach_pipeline};
pub(crate) use services::Services;
pub(crate) use settings::{attach_settings, legacy_settings_import};
#[cfg(test)]
pub(crate) use speakers::Speakers;
pub(crate) use stt::attach_stt;
use tauri::{AppHandle, Manager};
#[cfg(test)]
pub(crate) use vocabulary::Vocabulary;

use crate::database::manager::DatabaseManager;
use crate::features;
use crate::paths::AppPaths;
use crate::state::AppState;

/// Wires a newly opened database into the app: the legacy `AppState`, every
/// feature service, the pipeline and the `DbChanged` bridge to the UI.
///
/// Runs once, at startup or from the first-launch `initialize_fresh_database`
/// command. Feature commands called before that fail with Tauri's "state not
/// managed" error, as the legacy commands always have.
pub(crate) async fn attach_database(app: &AppHandle, manager: DatabaseManager) {
    let database = manager.database().clone();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    app.manage(AppState {
        db_manager: manager,
    });
    let cx = Context {
        app,
        database: &database,
        clock: &clock,
    };
    let organize = attach_local(&cx);
    let Some(settings) = app
        .try_state::<Arc<SettingsService>>()
        .map(|settings| Arc::clone(&settings))
    else {
        log::error!(target: "startup", "settings were not wired before the database; AI, templates and the profile are unavailable");
        return;
    };
    attach_library(&cx, &organize, settings).await;
}

/// Wires the services that need only the database: [`Services`], folders and
/// tags, then every [`attach::Phase::Local`] hook (speakers, the vocabulary,
/// the notepad, the sample meeting, the player, the egress log, ...) and the
/// `DbChanged` bridge. Returns folders and tags, whose auto-tagger the
/// pipeline needs.
fn attach_local(cx: &Context<'_>) -> Organize {
    Services::build(cx.database, cx.clock).manage(cx.app);
    let organize = Organize::build(cx.database, cx.clock, automations::tag_events(cx.app));
    organize.manage(cx.app);
    attach::run_local(cx);
    features::db::forward_changes(cx.app, cx.database.changes());
    organize
}

/// Wires everything over the settings and the LLM gateway: the meetings
/// list, search, the stage services, the [`attach::Phase::ReadModel`] hooks,
/// templates and the enhanced notes, then the pipeline and, once it runs,
/// the [`attach::Phase::Background`] hooks.
async fn attach_library(cx: &Context<'_>, organize: &Organize, settings: Arc<SettingsService>) {
    let (app, database, clock) = (cx.app, cx.database, cx.clock);
    meetings::attach_meetings(app, database, clock, (organize, &settings));
    let (gateway, search) = search::attach_gateway_and_search(app, database, &settings, clock);
    let wiring = ActionItemWiring {
        gateway: gateway.clone(),
        settings: Arc::clone(&settings),
        clock: Arc::clone(clock),
    };
    let stage_adapters = attach_stage_services(app, database, &wiring, (organize, search));
    attach::run_read_models(cx, &wiring);
    let deps = NoteDeps {
        settings: Arc::clone(&settings),
        gateway,
        clock: Arc::clone(clock),
    };
    // Templates are seeded before the pipeline starts: its notes stage reads them.
    let templates = attach_notes(app, database, deps.clone()).await;
    let notes_ai = NotesAi::build(database, &deps, templates);
    notes_ai.manage(app);
    let ai = StageServices {
        enhancer: notes_ai.enhancer(),
        titler: notes_ai.titler(),
        automatic: settings,
        adapters: stage_adapters,
    };
    attach_pipeline(app, database, clock, ai);
    attach::run_background(cx);
}

/// Builds and manages the services behind the pipeline's AI and export
/// stages: the auto-tagger, the exports (EXP-01/02), action items (AI-05),
/// insights (AI-06), the speaker identifier (CAP-04) and people and companies
/// (ORG-04); the search indexer (ORG-08) is already built.
fn attach_stage_services(
    app: &AppHandle,
    database: &Database,
    wiring: &ActionItemWiring,
    (organize, search): (&Organize, Arc<dyn SearchIndexing>),
) -> StageAdapters {
    let auto_tagger = organize.auto_tagger(wiring.gateway.clone(), Arc::clone(&wiring.settings));
    let exports = ExportServices::build(
        app,
        database,
        Arc::clone(&wiring.settings),
        Arc::clone(&wiring.clock),
    );
    let auto_export = exports.auto_exporter();
    exports.manage(app);
    let insights = insights::attach_insights(app, database, wiring);
    let automations =
        automations::attach_automations(app, database, wiring, (organize, insights.clone()));
    StageAdapters {
        auto_tagger,
        auto_export,
        action_items: action_items::attach_action_items(app, database, wiring),
        insights,
        automations,
        follow_ups: followups::attach_followups(app, database, wiring),
        identifier: speakers::identifier(database, wiring.settings.clone()),
        search,
        entities: entities::attach_entities(app, database, wiring.gateway.clone(), &wiring.clock),
    }
}

/// Wires templates and the profile (`notes.rs`) and prepares them; returns
/// the meeting template choice the enhanced notes are written with.
async fn attach_notes(
    app: &AppHandle,
    database: &Database,
    deps: NoteDeps,
) -> Arc<dyn deino_core::enhance::TemplateSource> {
    let notes = NoteServices::build(database, deps);
    let legacy_templates = AppPaths::resolve(app)
        .ok()
        .map(|paths| paths.legacy_templates());
    notes.prepare(legacy_templates.as_deref()).await;
    let templates = notes.template_source();
    notes.manage(app);
    templates
}

#[cfg(test)]
mod tests;
