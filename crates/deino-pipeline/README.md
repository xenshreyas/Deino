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
- The registry (`registry.rs`): `stages(&StagePorts)`, the one list of
  stages. **Adding a stage is one line there.**
- The stages (`stages/`): `legacy_summary` (order 50) over the
  `Summarizer` port, until NOT-02 replaces it.
- The queue: the `jobs` and `job_runs` tables (migration
  `20261009071429_fnd08_pipeline_jobs.sql` in `deino-db`), behind the
  `JobRepo` port and `SqliteJobRepo`.
- The runner (`runner/`): one worker per meeting, `MAX_CONCURRENT_STAGES` (2)
  stages at once across meetings, retries with clock-driven backoff
  (`RetryPolicy`, 3 attempts), per-meeting cancellation, and a `TaskTracker`
  that owns every task.
- `PipelineService`: `enqueue`, `rerun`, `status`, `cancel`, `start`
  (startup reconciliation), `shutdown`; and `PipelineTrigger`, the
  `MeetingStatusObserver` that enqueues meetings entering `processing`.
- `PipelineProgress` and the `ProgressSink` port.
- Fakes behind `test-support`: `FakeSleeper`, `RecordedProgress`,
  `RecordedSummaries`.

## Does Not Own

- Meetings and their status rules: `deino-core::meetings`
  (`MeetingLifecycleService`).
- IPC, the Tauri event and the legacy summary adapter: the app crate
  (`features/pipeline*`).
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
