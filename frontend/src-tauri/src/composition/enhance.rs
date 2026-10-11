//! Wiring for enhanced notes (NOT-02): the `EnhanceService` behind the
//! `enhance` plugin and the `enhance_notes` stage, and the `AutoTitler`
//! behind `auto_title`, over the SQLite repositories, the LLM gateway and
//! the meeting's template choice (NOT-04).

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::enhance::{
    AutoTitler, EnhanceError, EnhancePorts, EnhanceService, MaterialSources, PipelineRun,
    SqliteEnhancedNotesRepo, TemplateSource, TitleRun,
};
use deino_db::Database;
use deino_domain::{Clock, MeetingId};
use deino_llm::Gateway;
use deino_pipeline::{MeetingTitler, NotesEnhancer};
use tauri::{Manager, Runtime};

/// Why the notes stages skip when the gateway could not be wired.
const AI_UNAVAILABLE: &str = "AI is unavailable: the Keychain or the network could not start.";

/// The enhanced-notes services, or stand-ins that skip when there is no
/// gateway.
pub(crate) struct NotesAi {
    service: Option<Arc<EnhanceService>>,
    enhancer: Arc<dyn NotesEnhancer>,
    titler: Arc<dyn MeetingTitler>,
}

impl NotesAi {
    /// Builds the services over `database`, asking `gateway` and choosing
    /// templates through `templates`.
    pub(crate) fn build(
        database: &Database,
        gateway: Option<Arc<Gateway>>,
        templates: Arc<dyn TemplateSource>,
        clock: &Arc<dyn Clock>,
    ) -> Self {
        let Some(gateway) = gateway else {
            return Self {
                service: None,
                enhancer: Arc::new(Unwired),
                titler: Arc::new(Unwired),
            };
        };
        let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
        let sources = MaterialSources::sqlite(database);
        let titler = AutoTitler::new(
            sources.clone(),
            repo.clone(),
            gateway.clone(),
            clock.clone(),
        );
        let ports = EnhancePorts {
            sources,
            reader: repo.clone(),
            writer: repo,
            templates,
            gateway,
        };
        let service = Arc::new(EnhanceService::new(ports, Arc::clone(clock)));
        Self {
            enhancer: service.clone(),
            service: Some(service),
            titler: Arc::new(titler),
        }
    }

    /// The `enhance_notes` stage's port.
    pub(crate) fn enhancer(&self) -> Arc<dyn NotesEnhancer> {
        Arc::clone(&self.enhancer)
    }

    /// The `auto_title` stage's port.
    pub(crate) fn titler(&self) -> Arc<dyn MeetingTitler> {
        Arc::clone(&self.titler)
    }

    /// Registers the service as Tauri state, where the `enhance` commands
    /// find it.
    pub(crate) fn manage<R: Runtime>(&self, app: &impl Manager<R>) {
        if let Some(service) = &self.service {
            app.manage(Arc::clone(service));
        }
    }
}

/// The notes stages without a gateway: they skip, saying why.
struct Unwired;

#[async_trait]
impl NotesEnhancer for Unwired {
    async fn enhance(&self, _meeting: &MeetingId) -> Result<PipelineRun, EnhanceError> {
        Ok(PipelineRun::Skipped(AI_UNAVAILABLE))
    }
}

#[async_trait]
impl MeetingTitler for Unwired {
    async fn title(&self, _meeting: &MeetingId) -> Result<TitleRun, EnhanceError> {
        Ok(TitleRun::Skipped(AI_UNAVAILABLE))
    }
}
