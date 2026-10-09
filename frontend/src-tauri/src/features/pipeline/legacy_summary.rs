//! [`LegacySummarizer`]: the `legacy_summary` stage's bridge to the
//! legacy summary job.
//!
//! The one place the pipeline calls the summary code. It does what
//! `api_process_transcript` does for the meeting page (open a
//! `summary_processes` row, save the transcript to `transcript_chunks`, run
//! a `SummaryJob`, which asks `deino_core::summaries::SummaryGenerator`
//! through the LLM gateway), but awaits the job instead of spawning it, so
//! the stage succeeds or fails with it. The meeting page keeps reading the
//! summary from `summary_processes` as before. NOT-02's `enhance_notes`
//! replaces this file.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_core::settings::SettingsService;
use deino_core::templates::{GENERAL_TEMPLATE_ID, MeetingTemplateService};
use deino_domain::{Clock, ErrorCode, MeetingId};
use deino_pipeline::{StageError, StageOutcome, Summarizer, SummaryInput};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager, Runtime};
use tokio_util::sync::CancellationToken;

use crate::database::repositories::summary::SummaryProcessesRepository;
use crate::database::repositories::transcript_chunk::TranscriptChunksRepository;
use crate::summary::SummaryService;
use crate::summary::service::SummaryJob;

/// Characters per chunk and their overlap, as the meeting page requests them.
const CHUNK_SIZE: i32 = 40_000;
const CHUNK_OVERLAP: i32 = 1_000;

/// The summary provider and model chosen in Settings.
struct SummaryModel {
    provider: String,
    model: String,
}

/// Generates summaries through the legacy `SummaryService` job.
pub struct LegacySummarizer<R: Runtime> {
    app: AppHandle<R>,
    pool: SqlitePool,
    settings: Arc<SettingsService>,
    clock: Arc<dyn Clock>,
}

impl<R: Runtime> LegacySummarizer<R> {
    /// A generator writing through the legacy writer `pool`, with the
    /// provider and model chosen in `settings`.
    pub fn new(
        app: AppHandle<R>,
        pool: SqlitePool,
        settings: Arc<SettingsService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            app,
            pool,
            settings,
            clock,
        }
    }

    /// The provider and model to summarise with, by their legacy names.
    async fn model(&self) -> Result<SummaryModel, StageError> {
        let ai = self.settings.get().await.ai;
        let provider = serde_json::to_value(ai.provider)
            .ok()
            .and_then(|name| name.as_str().map(str::to_owned));
        match provider {
            Some(provider) if !ai.model.trim().is_empty() => Ok(SummaryModel {
                provider,
                model: ai.model,
            }),
            _ => Err(StageError::permanent(
                ErrorCode::Unavailable,
                "Choose a summary model in Settings, then retry.",
            )),
        }
    }

    /// Opens the summary process and saves the transcript it reads; returns
    /// the process's start time, which identifies it.
    async fn open(
        &self,
        request: &SummaryInput,
        model: &SummaryModel,
    ) -> Result<DateTime<Utc>, StageError> {
        let meeting = request.meeting_id.as_str();
        let started_at = self.clock.now();
        SummaryProcessesRepository::create_or_reset_process(&self.pool, meeting, started_at)
            .await
            .map_err(|_| internal("could not start the summary"))?;
        TranscriptChunksRepository::save_transcript_data(
            &self.pool,
            meeting,
            &request.transcript,
            &model.provider,
            &model.model,
            CHUNK_SIZE,
            CHUNK_OVERLAP,
        )
        .await
        .map_err(|_| internal("could not save the transcript for the summary"))?;
        Ok(started_at)
    }

    /// The template auto-selection picks for a meeting without one (NOT-04:
    /// rules, the default template, then the LLM classifier), else General.
    async fn auto_template(&self, meeting: &MeetingId) -> String {
        let Some(templates) = self.app.try_state::<Arc<MeetingTemplateService>>() else {
            return GENERAL_TEMPLATE_ID.to_owned();
        };
        match templates.auto_choice(meeting).await {
            Ok(choice) => choice.template.id.to_string(),
            Err(error) => {
                log::warn!(target: "pipeline", "no automatic template for {meeting}: {error}");
                GENERAL_TEMPLATE_ID.to_owned()
            }
        }
    }

    /// Runs the summary job to the end. When `cancel` fires, it cancels the
    /// job, which then stores itself as cancelled.
    async fn run(&self, request: SummaryInput, model: SummaryModel, process: Process) {
        let meeting = request.meeting_id.to_string();
        let legacy = SummaryService::register_cancellation_token(&meeting, process.started_at);
        let template_id = match request.template {
            Some(template) => template.to_string(),
            None => self.auto_template(&request.meeting_id).await,
        };
        let job = SummaryJob {
            pool: self.pool.clone(),
            meeting_id: meeting,
            started_at: process.started_at,
            cancellation: legacy.clone(),
            text: request.transcript,
            provider: model.provider,
            model: model.model,
            custom_prompt: String::new(),
            template_id,
        };
        let summary = SummaryService::process_transcript_background(self.app.clone(), job);
        tokio::pin!(summary);
        tokio::select! {
            () = &mut summary => {}
            () = process.cancel.cancelled() => {
                legacy.cancel();
                summary.await;
            }
        }
    }

    /// How the summary process that started at `started_at` ended.
    async fn outcome(
        &self,
        meeting: &MeetingId,
        started_at: DateTime<Utc>,
    ) -> Result<StageOutcome, StageError> {
        let process = SummaryProcessesRepository::get_summary_data(&self.pool, meeting.as_str())
            .await
            .map_err(|_| internal("could not read the summary result"))?
            .ok_or_else(|| internal("the summary process disappeared"))?;
        if process.start_time != Some(started_at) {
            return Ok(StageOutcome::Skipped(
                "A newer summary was started from the meeting page.",
            ));
        }
        match process.status.to_ascii_lowercase().as_str() {
            "completed" => Ok(StageOutcome::Done),
            "cancelled" => Err(StageError::Cancelled),
            // The legacy error text stays in the summary panel; it may quote the provider.
            _ => Err(StageError::retryable(
                ErrorCode::Llm,
                "The summary could not be generated; the summary panel shows why.",
            )),
        }
    }
}

/// One summary run: when it started (its ID) and what cancels it.
struct Process {
    started_at: DateTime<Utc>,
    cancel: CancellationToken,
}

#[async_trait]
impl<R: Runtime> Summarizer for LegacySummarizer<R> {
    async fn generate(
        &self,
        request: SummaryInput,
        cancel: CancellationToken,
    ) -> Result<StageOutcome, StageError> {
        let model = self.model().await?;
        let started_at = self.open(&request, &model).await?;
        let meeting = request.meeting_id.clone();
        self.run(request, model, Process { started_at, cancel })
            .await;
        self.outcome(&meeting, started_at).await
    }
}

/// A retryable failure of Deino's own storage.
fn internal(message: &str) -> StageError {
    StageError::retryable(ErrorCode::Internal, message)
}
