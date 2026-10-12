# 0042. Automations: a step registry, runs as pipeline work with one stored run per meeting, and shell scripts behind a switch

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** AUT-01 (Automations engine)

## Context

AUT-01 brings Circleback's automations (§10.1–§10.2) to a local app: "when
a meeting like this finishes, do these steps". An automation has a name,
switches (on, run automatically, stop at the first failed step), conditions
(AND within a group, OR across groups, none for every meeting) and an
ordered list of steps: generate an insight (AI-06), add a tag, add to a
folder, export a file (EXP-02), a macOS notification, a shell script with
the meeting as JSON on stdin (Muesli's post-meeting hooks), and later a
webhook (DEV-01) and provider steps (INT-01..07). It runs after a meeting
is processed, when a tag is applied (also to an upcoming meeting, which
arms it), and by hand: Test on latest meeting, Run on selected matching
meetings, and Rerun from a meeting's ⋯ menu. The run log shows each step's
result and error.

Four forces shape the design. Steps arrive from many cards over time, and
most of them (D12) are not built yet but must be visible. Steps write to
the outside world, so a retry must not post twice, and re-processing a
meeting must not repeat an automation. Work after a meeting is the
pipeline's job (§3.7): durable, retried, cancellable. And a shell script
runs arbitrary code with the user's permissions.

## Decision

**One `Step` per `ActionKind`, found in a `StepRegistry`.** `Action` is a
typed enum (one variant per kind, with its settings); a `Step` implements
`run(&StepContext, &Action) -> Result<StepOutput, StepError>` for one kind.
The registry is built in the composition root (`LocalStepPorts::registry`
for the local steps); a provider card adds an `Action` variant, its `Step`
and one `.with(..)`. The engine never names a step (Open/Closed). The
catalogue the picker shows lists every kind: kinds without a registered
step are `wip` and cannot be added (D12). A step sees the earlier steps'
outputs (`StepContext::earlier`), so an insight's answers reach a later
script's payload. External payloads are `AutomationPayload`
(`deino.automation` v1), assembled by EXP-01's `ExportAssembler` so the
JSON export and a script agree, with the fields each step chose.

**Conditions are AI-06's.** `deino_domain::conditions` gained AUT-01's
kinds (participants, title is/is not/regex, invitee emails and count,
domains all/not all, duration, calendar) and is the one rule engine; the
UI's condition editor moved to `features/conditions/`, shared by both.

**Runs: one stored run per automation and meeting, executed by the
pipeline.** `automation_runs` keeps the latest run of each automation on
each meeting (status `armed`, `pending`, `done`, `failed`, and the trigger:
`pipeline`, `tag_applied`, `manual`); `automation_run_steps` keeps each
step's status (`pending`, `done`, `failed`, `skipped`), a typed
`StepFailure`, a user-safe error and its output. The `automations` stage
(order 95, after `insights` at 85, so conditions and the insight step see
the finished meeting) decides and runs:

- every automation that runs on its own and matches gets a `pipeline` run
  **unless it already has a run on the meeting**, so re-processing never
  repeats it; only the user's Run or Rerun replaces a run;
- waiting runs (`pending` from a manual run, `armed` by a tag) run too; an
  armed run whose conditions no longer match is dropped;
- steps run in order; a failed step does not stop the rest unless the
  automation stops on failure (then the rest are `skipped`);
- a retryable failure (a rate-limited insight) leaves the step pending and
  fails the stage so the runner retries; done steps are not run again, so
  their side effects happen once. On the last attempt it is stored like
  any other failure.

Manual triggers store the request and schedule the meeting's `automations`
stage through `PipelineService::rerun` (the `AutomationScheduler` port);
they ignore the switches and conditions, as Circleback does. A tag applied
to a meeting gives a run to every automation that runs on its own, tests
for that tag and now matches, unless it already has a run there: `pending`
(and the stage scheduled) on a processed meeting, `armed` on one that is
upcoming or still processing.

**Shell scripts are off until the user turns them on.** The setting
`automations.shellScripts` (default `false`, "Allow shell scripts" on the
Automations page) is read by the step at every run, not when the
automation is saved, so turning it off stops every script at once. The
builder blocks adding a script step while it is off and warns on existing
ones. A script runs `/bin/sh -c <command>` in the meeting's folder
(`<recordings>/<meeting>/`) with the payload on stdin, in its own process
group; after 60 s the whole group is killed, so children cannot outlive it.
A zero exit is success; the tail of stdout (400 characters) is the step's
output and the tail of stderr goes into the failure's message. Neither is
logged. The payload and exported files go through the same NOT-09
redaction as every export while "Hide sensitive information" is on.

## Alternatives

- **Steps as a closed `match` in the engine.** Shorter today, but every
  provider card would edit the engine, and WIP kinds would need special
  cases in it.
- **An append-only run history.** A full history per meeting is more than
  the run log needs (the latest run per meeting, the latest runs per
  automation), and "has this automation run here" would need a query over
  it on every pipeline pass. Rerun replacing the run keeps that check a
  primary-key lookup.
- **Runs in their own task queue.** Would duplicate the pipeline's
  durability, retries, cancellation and restart reconciliation, which §3.7
  forbids.
- **Shell scripts always on, or approved per automation.** Always on makes
  a synced or shared automation a code-execution path; a per-automation
  approval is easy to click through and does not give one place to turn
  every script off.
- **A sandbox for scripts** (`sandbox-exec` profiles). Deprecated on macOS
  and would break the scripts users want (writing to their vault, calling
  CLIs). The switch, the timeout and the process group are the boundary;
  the script is the user's own code.

## Consequences

- A provider step is one module plus one registry line; its card wraps
  `IntegrationService::deliver` with a `run:<run>:<position>` idempotency
  key and maps retryable errors to `StepError` so the stage retries.
- Rerun is the only way to run an automation twice on a meeting; changing
  an automation does not re-run it on meetings it already ran on.
- Manual runs on a meeting being processed wait for its stage, like any
  pipeline work; the run log shows them as Queued.
- Script output is kept short in the database; anything larger belongs in
  a file the script writes.
