//! Wiring for templates (NOT-04) and the profile (NOT-05).
//!
//! Builds the template library, per-meeting template selection (with the
//! gateway classifier on route `tagging` when the gateway is up) and the
//! profile service over the SQLite repositories, and prepares them at
//! startup: the built-in catalogue is synced, the pre-NOT-04 custom template
//! files are imported once, and the "Me" person is made to exist.

use std::path::Path;
use std::sync::Arc;

use deino_core::enhance::TemplateSource;
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::people::SqlitePeopleRepo;
use deino_core::profile::ProfileService;
use deino_core::settings::SettingsService;
use deino_core::templates::{
    BuiltinSync, CATALOG, GatewayTemplateClassifier, LegacyTemplateFolder, MeetingTemplatePorts,
    MeetingTemplateService, NoClassifier, SqliteTemplateRepo, TemplateClassifier, TemplateService,
};
use deino_db::Database;
use deino_domain::Clock;
use deino_llm::Gateway;
use tauri::{Manager, Runtime};

/// What the note services need besides the database.
#[derive(Clone)]
pub(crate) struct NoteDeps {
    /// The settings (default template, profile).
    pub settings: Arc<SettingsService>,
    /// The gateway, when it could be wired; selection then has an LLM fallback.
    pub gateway: Option<Arc<Gateway>>,
    /// The time source.
    pub clock: Arc<dyn Clock>,
}

/// The template and profile services.
pub(crate) struct NoteServices {
    templates: Arc<TemplateService>,
    meeting_templates: Arc<MeetingTemplateService>,
    profile: Arc<ProfileService>,
    builtins: BuiltinSync,
}

impl NoteServices {
    /// Builds every note service over `database`.
    pub(crate) fn build(database: &Database, deps: NoteDeps) -> Self {
        let templates = Arc::new(SqliteTemplateRepo::new(database.clone()));
        let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
        let classifier: Arc<dyn TemplateClassifier> = match deps.gateway {
            Some(gateway) => Arc::new(GatewayTemplateClassifier::new(gateway)),
            None => Arc::new(NoClassifier),
        };
        let meeting_templates = MeetingTemplateService::new(MeetingTemplatePorts {
            templates: templates.clone(),
            facts: templates.clone(),
            meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
            settings: deps.settings.clone(),
            classifier,
            clock: deps.clock.clone(),
        });
        Self {
            templates: Arc::new(TemplateService::new(templates.clone(), deps.clock.clone())),
            meeting_templates: Arc::new(meeting_templates),
            profile: Arc::new(ProfileService::new(
                deps.settings,
                people.clone(),
                people,
                deps.clock.clone(),
            )),
            builtins: BuiltinSync::new(templates, deps.clock),
        }
    }

    /// Startup work (see the module docs). Each step logs its own failure
    /// and never blocks startup.
    pub(crate) async fn prepare(&self, legacy_templates: Option<&Path>) {
        if let Err(error) = self.builtins.run(&CATALOG).await {
            log::error!(target: "templates", "could not sync the built-in templates: {error}");
        }
        if let Some(dir) = legacy_templates {
            match self
                .templates
                .import_legacy(&LegacyTemplateFolder::new(dir))
                .await
            {
                Ok(0) => {}
                Ok(count) => {
                    log::info!(target: "templates", "imported {count} legacy template files");
                }
                Err(error) => {
                    log::warn!(target: "templates", "could not import legacy templates: {error}");
                }
            }
        }
        if let Err(error) = self.profile.sync_me().await {
            log::warn!(target: "profile", "could not update the user's person: {error}");
        }
    }

    /// The meeting template choice, which enhanced notes are written with.
    pub(crate) fn template_source(&self) -> Arc<dyn TemplateSource> {
        self.meeting_templates.clone()
    }

    /// Registers each service as Tauri state, where commands find it.
    pub(crate) fn manage<R: Runtime>(self, app: &impl Manager<R>) {
        app.manage(self.templates);
        app.manage(self.meeting_templates);
        app.manage(self.profile);
    }
}
