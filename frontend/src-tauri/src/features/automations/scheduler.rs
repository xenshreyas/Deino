//! [`PipelineScheduler`]: automations' [`AutomationScheduler`] over the
//! post-meeting pipeline (FND-08), which runs a meeting's `automations`
//! stage again.
//!
//! The pipeline is built after the automations service (its stage needs
//! the automations' engine), so the scheduler finds it in Tauri state when
//! a run is asked for, not when it is built.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::automations::AutomationScheduler;
use deino_domain::{AppError, ErrorClass, ErrorCode, MeetingId, StageId};
use deino_pipeline::PipelineService;
use tauri::{AppHandle, Manager, Runtime};

/// The stage that runs automations (`deino-pipeline`'s `stages/automations.rs`).
const AUTOMATIONS_STAGE: &str = "automations";

/// Schedules meetings' `automations` stage in the app's pipeline.
pub(crate) struct PipelineScheduler<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> PipelineScheduler<R> {
    /// A scheduler over the pipeline `app` manages.
    pub(crate) fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

#[async_trait]
impl<R: Runtime> AutomationScheduler for PipelineScheduler<R> {
    async fn schedule(&self, meeting: &MeetingId) -> Result<(), AppError> {
        let Some(pipeline) = self.app.try_state::<Arc<PipelineService>>() else {
            return Err(AppError::new(
                ErrorCode::Unavailable,
                "the post-meeting pipeline is not running",
            ));
        };
        let pipeline = Arc::clone(&pipeline);
        pipeline
            .rerun(meeting, Some(&StageId::new(AUTOMATIONS_STAGE)))
            .await
            .map(|_| ())
            .map_err(|error| AppError::new(error.code(), error.to_string()))
    }
}
