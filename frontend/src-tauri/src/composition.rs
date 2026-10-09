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
//! 3. [`attach_database`]: the database-backed [`Services`] and the
//!    post-meeting [`Pipeline`] (started in the background), once
//!    `deino.sqlite` is open (its legacy settings are imported on the way
//!    by [`legacy_settings_import`]), the egress log's table, the LLM
//!    gateway (`llm.rs`), and templates and the profile (`notes.rs`: the
//!    built-in catalogue is synced before the pipeline starts).
//!
//! A new feature adds its service to [`Services`]: one field, one line in
//! [`Services::build`] and one in [`Services::manage`].

mod detection;
mod diarize;
mod llm;
mod network;
mod notes;
mod stt;

use std::sync::Arc;

pub(crate) use detection::attach_detection;
use notes::{NoteDeps, NoteServices};
pub(crate) use stt::attach_stt;

use deino_core::diarization::TrackDiarizer;
use deino_core::meetings::{MeetingLifecycleService, MeetingService, SqliteMeetingRepo};
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::{
    JsonFileSettingsStore, LegacyFileImport, LegacySqliteImport, SettingsError, SettingsService,
};
use deino_db::Database;
use deino_domain::{Clock, SystemClock};
use deino_pipeline::{
    ClockSleeper, PipelineDeps, PipelineService, PipelineTrigger, ProgressSink, SqliteJobRepo,
    StagePorts, Summarizer, registry,
};
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
}

impl Services {
    /// Builds every service over `database`, reading time from `clock`.
    pub(crate) fn build(database: &Database, clock: &Arc<dyn Clock>) -> Self {
        let meeting_repo = Arc::new(SqliteMeetingRepo::new(database.clone()));
        Self {
            meetings: Arc::new(MeetingService::new(meeting_repo, Arc::clone(clock))),
        }
    }

    /// Registers each service as Tauri state, where commands find it.
    pub(crate) fn manage<R: Runtime>(self, app: &impl Manager<R>) {
        app.manage(self.meetings);
    }
}

/// What the pipeline gets from the app: where progress goes, the legacy
/// summary service behind the `legacy_summary` stage, and the diarization
/// engine behind `diarize`.
pub(crate) struct PipelineAdapters {
    pub(crate) progress: Arc<dyn ProgressSink>,
    pub(crate) summaries: Arc<dyn Summarizer>,
    pub(crate) diarizer: Arc<dyn TrackDiarizer>,
}

/// The meetings lifecycle and the post-meeting pipeline it starts (FND-08).
pub(crate) struct Pipeline {
    lifecycle: Arc<MeetingLifecycleService>,
    pipeline: Arc<PipelineService>,
}

impl Pipeline {
    /// Builds the pipeline over `database` with the registered stages, and
    /// subscribes it to the lifecycle so `processing` meetings enqueue.
    pub(crate) fn build(
        database: &Database,
        clock: &Arc<dyn Clock>,
        adapters: PipelineAdapters,
    ) -> Self {
        let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
        let lifecycle = Arc::new(MeetingLifecycleService::new(
            meetings.clone(),
            Arc::clone(clock),
        ));
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        let ports = StagePorts {
            segments: segments.clone(),
            meetings: meetings.clone(),
            summaries: adapters.summaries,
            attendees: Arc::new(SqlitePeopleRepo::new(database.clone())),
            segment_writer: segments.clone(),
            speakers: segments,
            diarizer: adapters.diarizer,
            clock: Arc::clone(clock),
        };
        let pipeline = Arc::new(PipelineService::new(PipelineDeps {
            jobs: Arc::new(SqliteJobRepo::new(database.clone())),
            stages: registry::stages(&ports),
            lifecycle: Arc::clone(&lifecycle),
            meetings,
            clock: Arc::clone(clock),
            sleeper: Arc::new(ClockSleeper::new(Arc::clone(clock))),
            progress: adapters.progress,
        }));
        lifecycle.subscribe(Arc::new(PipelineTrigger::new(&pipeline)));
        Self {
            lifecycle,
            pipeline,
        }
    }

    /// Starts the runner (reconciling the queue in the background) and
    /// registers the lifecycle service and the pipeline as Tauri state.
    pub(crate) fn start_and_manage<R: Runtime>(self, app: &impl Manager<R>) {
        self.pipeline.start();
        app.manage(self.lifecycle);
        app.manage(self.pipeline);
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
    Services::build(&database, &clock).manage(app);
    network::attach_network_log(app, &database);
    #[cfg(debug_assertions)]
    attach_fixture_seeder(app, &database);
    features::db::forward_changes(app, database.changes());
    let Some(settings) = app
        .try_state::<Arc<SettingsService>>()
        .map(|settings| Arc::clone(&settings))
    else {
        log::error!(target: "startup", "settings were not wired before the database; AI, templates and the profile are unavailable");
        return;
    };
    let gateway = llm::attach_llm(app, &database, &settings, clock.clone());
    let deps = NoteDeps {
        settings,
        gateway,
        clock: Arc::clone(&clock),
    };
    // Templates are seeded before the pipeline starts: its summary stage reads them.
    attach_notes(app, &database, deps).await;
    attach_pipeline(app, &database, &clock);
}

/// Wires templates and the profile (`notes.rs`) and prepares them.
async fn attach_notes(app: &AppHandle, database: &Database, deps: NoteDeps) {
    let notes = NoteServices::build(database, deps);
    let legacy_templates = AppPaths::resolve(app)
        .ok()
        .map(|paths| paths.legacy_templates());
    notes.prepare(legacy_templates.as_deref()).await;
    notes.manage(app);
}

/// Builds the pipeline with the app's adapters, starts it and manages it.
/// Needs the settings ([`attach_settings`] runs first), which name the
/// summary model the `legacy_summary` stage uses.
fn attach_pipeline(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        return log::error!(target: "pipeline", "settings are not loaded; the pipeline is off");
    };
    let diarizer = match diarize::track_diarizer(app) {
        Ok(diarizer) => diarizer,
        Err(error) => {
            return log::error!(target: "pipeline", "diarization could not start: {error}; the pipeline is off");
        }
    };
    let adapters = PipelineAdapters {
        diarizer,
        progress: Arc::new(features::pipeline::TauriPipelineProgress::new(app.clone())),
        summaries: Arc::new(features::pipeline::LegacySummarizer::new(
            app.clone(),
            database.writer().clone(),
            Arc::clone(&settings),
            Arc::clone(clock),
        )),
    };
    Pipeline::build(database, clock, adapters).start_and_manage(app);
}

/// Debug builds only: gives the `dev` plugin a seeder that writes the fixture
/// meetings into the app's database (Settings → Developer).
#[cfg(debug_assertions)]
fn attach_fixture_seeder(app: &AppHandle, database: &Database) {
    let seeder = deino_testkit::FixtureSeeder::new(database.clone());
    app.manage(Arc::new(seeder));
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

    #[tokio::test]
    async fn stopping_a_recording_runs_the_summary_stage_and_readies_the_meeting() {
        use deino_core::meetings::MeetingLifecycleRepo;
        use deino_domain::{MeetingEvent, MeetingId, MeetingStatus};
        use deino_pipeline::{RecordedProgress, RecordedSummaries};

        let database = test_db().await;
        let fake_clock = Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z")));
        recording_with_transcript(&database, &fake_clock).await;
        let summaries = Arc::new(RecordedSummaries::default());
        let adapters = PipelineAdapters {
            progress: Arc::new(RecordedProgress::default()),
            summaries: summaries.clone(),
            diarizer: Arc::new(deino_core::diarization::FakeTrackDiarizer::default()),
        };
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

        let requests = summaries.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].transcript, "[00:02] let's ship it");
        let state = SqliteMeetingRepo::new(database).state(&meeting).await;
        assert_eq!(state.unwrap().unwrap().status, MeetingStatus::Ready);
    }
}
