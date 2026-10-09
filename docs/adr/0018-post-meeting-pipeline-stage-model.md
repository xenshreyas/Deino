# 0018. Run post-meeting work as ordered, durable pipeline stages

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** FND-08

## Context

Everything that happens after a meeting (finalising audio, transcription
passes, diarisation, notes, action items, indexing, automations, exports;
PARITY_PLAN §4.2) used to be chained ad hoc: the meeting page started the
summary with `api_process_transcript`, which spawned a detached task. Twenty
cards will add steps, the app can quit or crash mid-step, LLM calls fail
transiently, and the user needs to see and retry what failed. §3.7 forbids
fire-and-forget `tokio::spawn`.

Spacedrive's job system (`core/src/infra/job/`, Apache-2.0) shows the
contract worth copying: named, resumable jobs, lifecycle hooks, and startup
reconciliation that leaves no zombie `running` rows
(`docs/references/architecture.md` (b) FND-08).

## Decision

`crates/deino-pipeline` runs a fixed, ordered list of **stages** per meeting:

- **Contract:** `#[async_trait] trait PipelineStage { fn id() -> StageId;
  fn order() -> u16; async fn run(&StageCtx, MeetingId) -> Result<StageOutcome,
  StageError>; }`. Methods, not associated consts, so stages live in a
  `Vec<Arc<dyn PipelineStage>>`. Stages are idempotent; `StageCtx` carries
  the attempt, a `CancellationToken`, the `Clock` and the progress sink;
  stage-specific ports are given when the registry builds the stage.
  `StageOutcome` is `Done` or `Skipped(&'static str)`; `StageError` is
  `Retryable`, `Permanent` (both carrying a user-safe `AppError`) or
  `Cancelled`.
- **Registry:** `registry::stages(&StagePorts)` is the one list. Adding a
  stage is one line there (plus a `StagePorts` field for a new port).
- **Durable queue:** one `jobs` row per (meeting, stage) holds the stage's
  state in the current run (`pending`, `running`, `succeeded`, `skipped`,
  `failed`, `cancelled`, attempts, retry time, user-safe message);
  `job_runs` keeps every attempt. Every status change is a compare-and-set
  in SQLite before the runner acts on it.
- **Order and failure:** a meeting's stages run one at a time, in order. A
  stage that fails for good (or is cancelled) stops the stages after it,
  which stay pending; re-running the failed stage alone continues the
  pipeline from there. When nothing is left to run, the runner moves the
  meeting to `ready` or `failed` through `MeetingLifecycleService`.
- **Concurrency:** one worker per meeting (a slot map with a rescan flag, so
  a wake is never lost) and a global semaphore of 2 permits held per
  attempt, so a meeting waiting out a backoff blocks nobody.
- **Retries:** 3 attempts, backoff 10 s then 20 s (capped at 5 min),
  computed from the injected `Clock` and waited out through a `Sleeper`
  port, so tests use `FakeSleeper` and never sleep. The retry time is
  stored, so backoff survives a restart.
- **Cancellation:** per meeting (the cancel holds until the worker has acted
  on it) and on shutdown. A cancelled stage gets a 2 s grace period to stop
  and clean up before it is abandoned. Shutdown leaves the interrupted job
  pending without counting the attempt.
- **Startup reconciliation:** delete jobs of unregistered stages; settle
  `running` rows a crash left (the attempt counts, so a stage that crashes
  the app eventually fails instead of looping); then open the runner,
  schedule `processing` meetings that have no jobs and wake every meeting
  with work. Workers wait for this before starting anything.
- **Trigger:** `MeetingLifecycleService` gained observers
  (`MeetingStatusObserver`); `PipelineTrigger` enqueues a meeting when it
  enters `processing`. Enqueueing is a no-op while the meeting already has
  work queued or running, which lets `rerun` schedule one stage and then
  move a `ready`/`failed` meeting back to `processing` without restarting
  the whole pipeline.
- **IPC:** the `pipeline` plugin exposes `status(meetingId)` and
  `rerun(meetingId, stage?)`, and the typed `PipelineProgress` event, which
  the meeting page's status strip uses to refetch.

## Alternatives

- **One job row per meeting with a stage cursor.** Simpler queue, but
  re-running one stage, showing each stage's status and keeping per-stage
  messages all need a second table anyway.
- **A generic job system (spacedrive's full manager, apalis, a DAG).** Typed
  inputs, serialized state, pause/resume and dependency graphs are more than
  a fixed per-meeting sequence needs, and every stage would pay for them.
- **Keep running later stages after a failure.** Most later stages read
  what earlier ones wrote (notes need the transcript); a stage that can run
  without its predecessor returns `Skipped` instead.
- **Sleep with `tokio::time` directly.** Untestable without paused time and
  a fake clock that agree; the `Sleeper` port keeps time in one place.

## Consequences

- New post-meeting work is a stage: no card adds background tasks.
- The `legacy_summary` stage (order 50) bridges to the inherited summary
  service through the `Summarizer` port until NOT-02's
  `enhance_notes`; its jobs are pruned automatically once it is removed.
- A stage must be idempotent and keep its messages free of meeting content.
- Per-meeting cancellation exists in Rust but has no command yet; meeting
  deletion and a UI cancel can call `PipelineService::cancel`.
- Revisit the stop-on-failure rule if stages appear that are independent of
  everything before them; a per-stage `requires` list would then replace
  the strict order.
