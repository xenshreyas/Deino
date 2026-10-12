//! Wiring for automations (AUT-01): the service the `automations` plugin
//! calls and the engine behind the pipeline's `automations` stage, with the
//! local steps over the insights generator, the tag and folder services,
//! the export assembler (redacted like every export, NOT-09), the file
//! system, system notifications, the settings (shell scripts) and the
//! recordings folder, plus DEV-01's webhook step over the webhooks service.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::automations::steps::RecordingFolders;
use deino_core::automations::{
    AssembledMeetings, AutomationEngine, AutomationPorts, AutomationRunner, AutomationService,
    EngineDeps, FactsSource, LocalStepPorts, MeetingSource, SqliteAutomationRepo, StepRegistry,
};
use deino_core::export::FsVault;
use deino_core::insights::{InsightGenerator, SqliteInsightRepo};
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::tags::TagEvents;
use deino_core::webhooks::{WebhookService, WebhookStep};
use deino_db::Database;
use tauri::{AppHandle, Manager};

use super::Organize;
use super::action_items::ActionItemWiring;
use super::export::redacting_assembler;
use crate::features::action_items::SystemNotifier;
use crate::features::automations::{AutomationTagEvents, PipelineScheduler};
use crate::features::tags::TauriTagEvents;
use crate::paths::AppPaths;

/// Builds and manages the automations service, and returns the engine for
/// the pipeline.
pub(super) fn attach_automations(
    app: &AppHandle,
    database: &Database,
    wiring: &ActionItemWiring,
    local: (&Organize, Arc<dyn InsightGenerator>),
) -> Arc<dyn AutomationRunner> {
    let repo = Arc::new(SqliteAutomationRepo::new(database.clone()));
    let insights = Arc::new(SqliteInsightRepo::new(database.clone()));
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let facts = FactsSource::new(insights, wiring.settings.clone());
    let steps = local_steps(app, database, wiring, local);
    app.manage(Arc::new(AutomationService::new(AutomationPorts {
        automations: repo.clone(),
        runs: repo.clone(),
        facts: facts.clone(),
        meetings,
        steps: steps.clone(),
        scheduler: Arc::new(PipelineScheduler::new(app.clone())),
        clock: wiring.clock.clone(),
    })));
    Arc::new(AutomationEngine::new(EngineDeps {
        automations: repo.clone(),
        runs: repo,
        facts,
        steps,
        clock: wiring.clock.clone(),
    }))
}

/// The tags service's event port: Tauri's `MeetingTagged` event, and the
/// automations' tag trigger for every tag newly applied.
pub(super) fn tag_events(app: &AppHandle) -> Arc<dyn TagEvents> {
    Arc::new(AutomationTagEvents::new(
        Arc::new(TauriTagEvents::new(app.clone())),
        app.clone(),
    ))
}

/// Every local step over the app's services and adapters.
fn local_steps(
    app: &AppHandle,
    database: &Database,
    wiring: &ActionItemWiring,
    (organize, generator): (&Organize, Arc<dyn InsightGenerator>),
) -> StepRegistry {
    let insights = Arc::new(SqliteInsightRepo::new(database.clone()));
    let meetings: Arc<dyn MeetingSource> = Arc::new(AssembledMeetings::new(
        redacting_assembler(database, wiring.settings.clone()),
        Arc::new(SqliteMeetingRepo::new(database.clone())),
    ));
    let steps = LocalStepPorts {
        insight_defs: insights.clone(),
        insight_runs: insights.clone(),
        generator,
        tags: organize.tags(),
        filer: organize.filing(),
        meetings: meetings.clone(),
        records: insights,
        vault: Arc::new(FsVault),
        notifier: Arc::new(SystemNotifier::new(app.clone())),
        shell_switch: wiring.settings.clone(),
        folders: Arc::new(RecordingFolders(recordings(app))),
        clock: wiring.clock.clone(),
    }
    .registry();
    webhook_step(app, steps, meetings)
}

/// `steps` with DEV-01's webhook step, when the webhooks are wired.
fn webhook_step(
    app: &AppHandle,
    steps: StepRegistry,
    meetings: Arc<dyn MeetingSource>,
) -> StepRegistry {
    match app.try_state::<Arc<WebhookService>>() {
        Some(service) => steps.with(Arc::new(WebhookStep::new(
            WebhookService::clone(&service),
            meetings,
        ))),
        None => steps,
    }
}

/// The folder every meeting's folder lives in: a script's working
/// directory is `<recordings>/<meeting>/`.
fn recordings(app: &AppHandle) -> PathBuf {
    match AppPaths::resolve(app) {
        Ok(paths) => paths.recordings(),
        Err(error) => {
            log::error!(target: "automations", "app folders unavailable: {error}; scripts run in the temporary folder");
            std::env::temp_dir()
        }
    }
}
