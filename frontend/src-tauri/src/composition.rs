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
//! 3. [`attach_database`]: the database-backed [`Services`], the
//!    post-meeting [`Pipeline`] (started in the background) and the live
//!    recorder (`recording.rs`, which also recovers interrupted recordings), once
//!    `deino.sqlite` is open (its legacy settings are imported on the way
//!    by [`legacy_settings_import`]), the egress log's table, the LLM
//!    gateway (`llm.rs`), templates and the profile (`notes.rs`: the
//!    built-in catalogue is synced before the pipeline starts), and the
//!    enhanced notes (`enhance.rs`, NOT-02).
//!
//! A new feature adds its service to [`Services`]: one field, one line in
//! [`Services::build`] and one in [`Services::manage`].

mod action_items;
mod browser;
mod calendar;
mod detection;
mod developer;
mod diarize;
mod enhance;
mod entities;
mod export;
mod insights;
mod integrations;
mod llm;
mod network;
mod notepad;
mod notes;
pub(crate) mod onboarding;
mod organize;
mod pipeline;
mod player;
mod recording;
mod search;
mod speakers;
mod stt;
mod transcript;
mod vocabulary;

use std::sync::Arc;

use action_items::ActionItemWiring;
pub(crate) use calendar::CalendarSchedule;
pub(crate) use detection::attach_detection;
pub(crate) use developer::attach_developer;
use enhance::NotesAi;
use export::ExportServices;
use notes::{NoteDeps, NoteServices};
pub(crate) use onboarding::attach_permissions;
pub(crate) use organize::Organize;
#[cfg(test)]
use pipeline::{Pipeline, PipelineAdapters};
use pipeline::{StageAdapters, StageServices, attach_pipeline};
pub(crate) use speakers::Speakers;
pub(crate) use stt::attach_stt;
pub(crate) use vocabulary::Vocabulary;

use deino_core::meetings::{MeetingService, SqliteMeetingRepo};
use deino_core::search::SearchIndexing;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::{
    JsonFileSettingsStore, LegacyFileImport, LegacySqliteImport, SettingsError, SettingsService,
};
use deino_core::transcript::TranscriptService;
use deino_db::Database;
use deino_domain::{Clock, SystemClock};
use deino_secrets::{KeychainSecretStore, SecretStore};
use tauri::{AppHandle, Manager, Runtime};

use crate::database::manager::DatabaseManager;
use crate::features;
use crate::features::settings::TauriSettingsEvents;
use crate::paths::AppPaths;
use crate::state::AppState;

/// The root HTTP client, for tests that talk to a loopback server.
#[cfg(test)]
pub(crate) use network::root_client;
pub(crate) use network::{attach_network, model_downloads};

/// Loads the settings, moves the legacy JSON stores into them, opens the
/// Keychain, and manages `Arc<SettingsService>` and `Arc<dyn SecretStore>`.
///
/// Runs first at startup. Keychain items carry the bundle identifier as
/// their service (`com.xenshreyas.deino`), so a build with another
/// identifier never sees this app's keys.
pub(crate) async fn attach_settings(
    app: &AppHandle,
    paths: &AppPaths,
) -> Result<(), SettingsError> {
    let store = Arc::new(JsonFileSettingsStore::new(paths.settings_file()));
    let events = Arc::new(TauriSettingsEvents::new(app.clone()));
    let settings = Arc::new(SettingsService::load(store, events).await?);
    // A store that cannot be imported keeps its defaults; it never blocks startup.
    if let Err(error) = LegacyFileImport::new(paths.data()).run(&settings).await {
        log::warn!(target: "settings", "could not import the legacy settings files: {error}");
    }
    let secrets: Arc<dyn SecretStore> =
        Arc::new(KeychainSecretStore::new(app.config().identifier.clone()));
    app.manage(settings);
    app.manage(secrets);
    Ok(())
}

/// The data migration that moves the legacy SQLite settings and keys into
/// the settings file and the Keychain before their tables are dropped.
/// `None` until [`attach_settings`] has run.
pub(crate) fn legacy_settings_import<R: Runtime>(app: &AppHandle<R>) -> Option<LegacySqliteImport> {
    let settings = app.try_state::<Arc<SettingsService>>()?;
    let secrets = app.try_state::<Arc<dyn SecretStore>>()?;
    Some(LegacySqliteImport::new(
        Arc::clone(&settings),
        Arc::clone(&secrets),
    ))
}

/// Every feature service backed by the app's database.
pub(crate) struct Services {
    meetings: Arc<MeetingService>,
    transcript: Arc<TranscriptService>,
}

impl Services {
    /// Builds every service over `database`, reading time from `clock`.
    pub(crate) fn build(database: &Database, clock: &Arc<dyn Clock>) -> Self {
        let meeting_repo = Arc::new(SqliteMeetingRepo::new(database.clone()));
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        let transcript = TranscriptService::new(segments);
        Self {
            meetings: Arc::new(MeetingService::new(meeting_repo, Arc::clone(clock))),
            transcript: Arc::new(transcript),
        }
    }

    /// Registers each service as Tauri state, where commands find it.
    pub(crate) fn manage<R: Runtime>(self, app: &impl Manager<R>) {
        app.manage(self.meetings);
        app.manage(self.transcript);
    }
}

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
    let organize = attach_local(app, &database, &clock);
    let Some(settings) = app
        .try_state::<Arc<SettingsService>>()
        .map(|settings| Arc::clone(&settings))
    else {
        log::error!(target: "startup", "settings were not wired before the database; AI, templates and the profile are unavailable");
        return;
    };
    let (gateway, search) = search::attach_gateway_and_search(app, &database, &settings, &clock);
    let wiring = ActionItemWiring {
        gateway: gateway.clone(),
        settings: Arc::clone(&settings),
        clock: Arc::clone(&clock),
    };
    let stage_adapters = attach_stage_services(app, &database, &wiring, (&organize, search));
    let deps = NoteDeps {
        settings: Arc::clone(&settings),
        gateway: gateway.clone(),
        clock: Arc::clone(&clock),
    };
    // Templates are seeded before the pipeline starts: its notes stage reads them.
    let templates = attach_notes(app, &database, deps.clone()).await;
    let notes_ai = NotesAi::build(&database, &deps, templates);
    notes_ai.manage(app);
    let ai = StageServices {
        enhancer: notes_ai.enhancer(),
        titler: notes_ai.titler(),
        automatic: settings,
        adapters: stage_adapters,
    };
    attach_pipeline(app, &database, &clock, ai);
    attach_background(app, &database, &clock);
}

/// Wires the services that need only the database: [`Services`], folders and
/// tags, speakers (CAP-04), the vocabulary (TRN-05), the notepad, the sample
/// meeting (FND-11), the player (TRN-03), the egress log, the fixture seeder
/// (debug builds) and the `DbChanged` bridge. Returns folders and tags,
/// whose auto-tagger the pipeline needs once the gateway is up.
fn attach_local(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) -> Organize {
    Services::build(database, clock).manage(app);
    let tag_events = Arc::new(features::tags::TauriTagEvents::new(app.clone()));
    let organize = Organize::build(database, clock, tag_events);
    organize.manage(app);
    Speakers::build(database, clock).manage(app);
    Vocabulary::build(database, clock).manage(app);
    notepad::attach_notepad(app, database, clock);
    onboarding::attach_sample(app, database, clock);
    player::attach_player(app, database);
    network::attach_network_log(app, database);
    #[cfg(debug_assertions)]
    developer::attach_fixture_seeder(app, database);
    features::db::forward_changes(app, database.changes());
    organize
}

/// Wires the services that run on their own once the pipeline is up: the
/// live recorder, the transcript edits (their re-index queue is search's),
/// calendar sync (its scheduler starts here) and the integrations service.
fn attach_background(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    recording::attach_recording(app, database);
    transcript::attach_transcript_edits(app, database, clock);
    calendar::attach_calendar(app, database, clock);
    integrations::attach_integrations(app, database, clock);
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
    StageAdapters {
        auto_tagger,
        auto_export,
        action_items: action_items::attach_action_items(app, database, wiring),
        insights: insights::attach_insights(app, database, wiring),
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

/// Wires the debug-only reference feature over its private in-memory database.
#[cfg(debug_assertions)]
pub(crate) async fn attach_example(app: &AppHandle) -> Result<(), deino_db::DbError> {
    use deino_core::_example::{ExampleService, MIGRATOR, SqliteExampleNoteRepo};
    use deino_db::DbLocation;

    let database = Database::open(&DbLocation::Memory, &MIGRATOR).await?;
    let repo = Arc::new(SqliteExampleNoteRepo::new(database.clone()));
    let events = Arc::new(features::_example::TauriExampleEvents::new(app.clone()));
    app.manage(Arc::new(ExampleService::new(
        repo,
        events,
        Arc::new(SystemClock),
    )));
    features::db::forward_changes(app, database.changes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use deino_core::segments::SqliteSegmentRepo;
    use deino_testkit::{FakeClock, at, seed, test_db};

    use super::*;

    #[tokio::test]
    async fn services_read_through_the_database_they_are_built_over() {
        let database = test_db().await;
        seed::meeting(
            &database,
            "meeting-1",
            "Standup",
            at("2026-10-08T09:00:00Z"),
        )
        .await;
        let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z")));

        let services = Services::build(&database, &clock);

        let meetings = services.meetings.list().await.unwrap();
        assert_eq!(meetings.len(), 1);
    }

    /// Records `meeting-1` with one transcript segment, still recording.
    async fn recording_with_transcript(database: &Database, clock: &FakeClock) {
        use deino_core::meetings::{MeetingLifecycleRepo, MeetingTitle, NewMeeting};
        use deino_core::segments::{NewSegment, SegmentWriter};
        use deino_domain::{Channel, MeetingId, MeetingSource, MeetingStatus, Millis, SegmentId};

        let meeting = NewMeeting {
            id: MeetingId::new("meeting-1"),
            title: MeetingTitle::parse("Standup").unwrap(),
            status: MeetingStatus::Recording,
            source: MeetingSource::Live,
            created_at: at("2026-10-08T09:00:00Z"),
            started_at: None,
            folder_path: None,
        };
        SqliteMeetingRepo::new(database.clone())
            .create(&meeting)
            .await
            .unwrap();
        let segment = NewSegment {
            id: SegmentId::generate(clock),
            meeting_id: meeting.id,
            channel: Channel::Mixed,
            speaker_id: None,
            start: Millis(2_000),
            end: Millis(3_000),
            text: "let's ship it".into(),
            words: Vec::new(),
        };
        SqliteSegmentRepo::new(database.clone())
            .append(&[segment])
            .await
            .unwrap();
    }

    /// The recorded stage services the test reads back.
    struct Recorded {
        enhancer: Arc<deino_pipeline::RecordedEnhancer>,
        titler: Arc<deino_pipeline::RecordedTitler>,
        auto_tags: Arc<deino_pipeline::RecordedAutoTags>,
    }

    /// Pipeline adapters over fakes, recording what the AI stages were asked.
    fn recorded_adapters() -> (PipelineAdapters, Recorded) {
        let recorded = Recorded {
            enhancer: Arc::default(),
            titler: Arc::default(),
            auto_tags: Arc::default(),
        };
        let adapters = PipelineAdapters {
            progress: Arc::new(deino_pipeline::RecordedProgress::default()),
            diarizer: Arc::new(deino_core::diarization::FakeTrackDiarizer::default()),
            ai: StageServices {
                enhancer: recorded.enhancer.clone(),
                titler: recorded.titler.clone(),
                automatic: Arc::new(deino_pipeline::FixedAutomaticNotes(true)),
                adapters: StageAdapters {
                    auto_tagger: recorded.auto_tags.clone(),
                    auto_export: Arc::new(deino_pipeline::ScriptedAutoExporter::default()),
                    action_items: Arc::new(
                        deino_core::action_items::FakeActionItemExtractor::default(),
                    ),
                    insights: Arc::new(deino_pipeline::stages::RecordedInsights::default()),
                    identifier: Arc::new(deino_pipeline::RecordedIdentifications::default()),
                    search: Arc::new(deino_pipeline::RecordedIndexing::default()),
                    entities: Arc::new(deino_pipeline::RecordedEntityUpdates::default()),
                },
            },
            transcript: pipeline::TranscriptStages {
                transcriber: Arc::new(deino_core::final_transcription::ScriptedTracks::default()),
                repass: Arc::new(deino_pipeline::FixedRepass(false)),
                vocab_fixer: Arc::new(deino_pipeline::RecordedFixer::default()),
            },
            recordings: std::path::PathBuf::from("no-recordings"),
        };
        (adapters, recorded)
    }

    #[tokio::test]
    async fn stopping_a_recording_runs_the_notes_stages_and_readies_the_meeting() {
        use deino_core::meetings::MeetingLifecycleRepo;
        use deino_domain::{MeetingEvent, MeetingId, MeetingStatus};

        let database = test_db().await;
        let fake_clock = Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z")));
        recording_with_transcript(&database, &fake_clock).await;
        let (
            adapters,
            Recorded {
                enhancer,
                titler,
                auto_tags,
            },
        ) = recorded_adapters();
        let clock: Arc<dyn Clock> = fake_clock;
        let wired = Pipeline::build(&database, &clock, adapters);
        wired.pipeline.reconcile().await.unwrap();
        let meeting = MeetingId::new("meeting-1");

        wired
            .lifecycle
            .transition(&meeting, MeetingEvent::StopRecording)
            .await
            .unwrap();
        wired.pipeline.until_idle().await;

        let once = std::slice::from_ref(&meeting);
        assert_eq!(enhancer.meetings(), once);
        assert_eq!(titler.meetings(), once);
        assert_eq!(auto_tags.meetings(), once);
        let state = SqliteMeetingRepo::new(database).state(&meeting).await;
        assert_eq!(state.unwrap().unwrap().status, MeetingStatus::Ready);
    }
}
