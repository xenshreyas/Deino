//! [`PipelineScheduler`]: insights' [`RunScheduler`] over the post-meeting
//! pipeline (FND-08), which runs a meeting's `insights` stage again.
//!
//! The pipeline is built after the insights service (its stage needs the
//! insights' generator), so the scheduler finds it in Tauri state when a
//! run is asked for, not when it is built.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::insights::RunScheduler;
use deino_domain::{AppError, ErrorClass, ErrorCode, MeetingId, StageId};
use deino_pipeline::PipelineService;
use tauri::{AppHandle, Manager, Runtime};

/// The stage that runs insights (`deino-pipeline`'s `stages/insights.rs`).
const INSIGHTS_STAGE: &str = "insights";

/// Schedules meetings' `insights` stage in the app's pipeline.
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
impl<R: Runtime> RunScheduler for PipelineScheduler<R> {
    async fn schedule(&self, meeting: &MeetingId) -> Result<(), AppError> {
        let Some(pipeline) = self.app.try_state::<Arc<PipelineService>>() else {
            return Err(AppError::new(
                ErrorCode::Unavailable,
                "the post-meeting pipeline is not running",
            ));
        };
        let pipeline = Arc::clone(&pipeline);
        pipeline
            .rerun(meeting, Some(&StageId::new(INSIGHTS_STAGE)))
            .await
            .map(|_| ())
            .map_err(|error| AppError::new(error.code(), error.to_string()))
    }
}
