# deino-pipeline

## Role

The post-meeting pipeline (PARITY_PLAN §4.2, ADR 0018): a durable,
SQLite-backed job runner that runs every registered stage for a meeting once
it enters `processing`, then moves the meeting to `ready` or `failed`.

## Owns

- The stage contract (`stage.rs`): `PipelineStage` (`id`, `order`, `run`),
  `StageCtx` (attempt, `CancellationToken`, clock, progress sink),
  `StageOutcome` (`Done`, `Skipped`) and `StageError` (`is_retryable`).
  Modelled on spacedrive's job contract (Apache-2.0).
- The registry (`registry.rs`): `stages(&StagePorts)` builds every stage in
  `STAGES`, sorted by order. **A stage registers itself** with
  `crate::registry::register_stage!(MyStage)` in its own file (REG-01).
- The stages (`stages/`): `finalize_audio` (order 10, FND-05b) over
  `deino_audio::{recover_recording, finalize_recording}` on a blocking
  thread, storing the mic, system and mixed paths with `set_audio` (skipped
  for meetings without a live recording folder); `final_transcription`
  (order 20, CAP-08) over `deino_core::final_transcription` and the
  `TrackTranscriber` port (the re-pass setting through `RepassSetting`;
  ElevenLabs through the `CloudChoice` port when the user opted in, CAP-14,
  retried on 429/5xx/network, then falling back to the Mac with an
  "ElevenLabs transcription failed: …" skip);
  `diarize` (order 30, CAP-03) over
  `deino_core::diarization::DiarizeService` (with the attendees' voiceprints);
  `identify_speakers` (order 35, CAP-04) over
  `deino_core::diarization::SpeakerIdentifier`; `vocab_fixups` (order 40,
  TRN-05) over the `VocabFixer` port (`deino_core::vocabulary::FixupService`);
  `enhance_notes` (order 50, NOT-02)
  over `deino_core::enhance::EnhanceService` (skipped when
  `Settings::ai.auto_summary`, "Generate notes automatically", is off; the
  meeting page's Enhance button writes notes on demand); `extract_action_items`
  (order 60, AI-05) over
  `deino_core::action_items::extraction::ActionItemExtractor`; `auto_title`
  (order 65, NOT-02) over `deino_core::enhance::AutoTitler`; `auto_tag`
  (order 70, ORG-06)
  over `deino_core::tags::AutoTagger`; `auto_folder` (order 75, ORG-05) over
  `deino_core::folders::AutoFiler`; `index` (order 80, ORG-08 and AI-01) over
  `deino_core::search::SearchIndexing` (keyword refresh and embeddings; skipped
  with a reason while the embedding model is not downloaded); `insights` (order 85,
  AI-06) over `deino_core::insights::InsightGenerator` (skipped with a reason when
  no insight applies; the last attempt stores failures on the runs);
  `update_entities`
  (order 90, ORG-04) over `deino_core::entities::EntityUpdater` (links named
  speakers, refreshes attendees' and companies' briefings); `automations`
  (order 95, AUT-01, ADR 0042) over `deino_core::automations::AutomationRunner`
  (matching automations and waiting runs; a retryable step fails the stage
  so it retries, done steps are not run again); `auto_export`
  (order 99, EXP-02) over `deino_core::export::AutoExporter`.
- The queue: the `jobs` and `job_runs` tables (migration
  `20261009071429_fnd08_pipeline_jobs.sql` in `deino-db`), behind the
  `JobRepo` port and `SqliteJobRepo`.
- The runner (`runner/`): one worker per meeting, `MAX_CONCURRENT_STAGES` (2)
  stages at once across meetings, retries with clock-driven backoff
  (`RetryPolicy`, 3 attempts), per-meeting cancellation, and a `TaskTracker`
  that owns every task.
- `PipelineService`: `enqueue`, `rerun`, `rerun_from` (a stage and every
  later one), `status`, `cancel`, `start`
  (startup reconciliation), `shutdown`; and `PipelineTrigger`, the
  `MeetingStatusObserver` that enqueues meetings entering `processing`.
- `PipelineProgress` and the `ProgressSink` port.
- Fakes behind `test-support`: `FakeSleeper`, `RecordedProgress`,
  `RecordedEnhancer`, `RecordedTitler`, `FixedAutomaticNotes`.

## Does Not Own

- Meetings and their status rules: `deino-core::meetings`
  (`MeetingLifecycleService`).
- IPC and the Tauri event: the app crate (`features/pipeline*`).
- How notes are written: `deino_core::enhance` (the stages only call it).
- What a stage does: each stage's card (§4.2 lists the planned orders).

## Invariants

- No dependency on `tauri`, and no raw `tokio::spawn`: every task runs on
  the runner's `TaskTracker`.
- A meeting's stages run one at a time, in registry order; a failed or
  cancelled stage stops the stages after it until it is re-run.
- Every state change is in SQLite before the runner acts on it; status
  changes are compare-and-set.
- Time comes from the injected `Clock`; waits go through the `Sleeper` port.
- After startup reconciliation there is no `running` row that no worker
  owns, and no job of an unregistered stage.
- Stage messages (skip reasons, failures) are user-safe: never meeting
  content. Logs carry IDs, stages and those messages only.
