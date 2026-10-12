# AUT-01 handoff (agent A → agent B)

Branch `feat/AUT-01-automations`. Phase A (engine, local steps, tests, functional UI) is done.

## Done
- **Conditions** (`crates/deino-domain/src/conditions.rs` + `conditions/{holds,normalize}.rs`): every card condition
  (tag exclude, participants in/ex, title is/is not/contains/lacks/regex, invitee email, domains are all/not all,
  invitee count =/>/<, external, template, duration >, calendar). 30-case table test. `MeetingFacts` gained
  participants, invitee emails, duration, calendar; AI-06's loader (`insights/facts.rs`, `meeting_facts.sql`) fills them.
- **Schema** `20261012022631_aut01_automations.sql`: `automations`, `automation_runs` (one latest run per automation
  and meeting), `automation_run_steps` (per-step status, typed `StepFailure`, output JSON). `Table` variants added.
- **Core** `crates/deino-core/src/automations/`: `Action`/`ActionKind`/`PayloadFields`, `Step` trait + `StepRegistry`,
  catalogue (provider steps + webhook = WIP, D12), local steps in `steps/` (insight, tag, folder, export, notify, shell),
  `AutomationEngine` (stage rules: idempotent, resume after done steps, stop-on-failure, retryable vs stored),
  `AutomationService` (CRUD, duplicate, run, test on latest, tag trigger with arming, meeting menu, matching).
- **Shell step**: `/bin/sh -c`, JSON payload on stdin, cwd `<recordings>/<meeting>/`, 60 s timeout, own process
  group killed with `rustix::kill_process_group`; gated by `settings.automations.shellScripts` (default off).
- **Pipeline** stage `automations` at 95 (`deino-pipeline/src/stages/automations.rs`), registry + `StagePorts.automations`.
- **App**: plugin `automations` (11 commands, manifest/lib/capabilities/features), `composition/automations.rs`,
  `PipelineScheduler` (reruns stage `automations`), `AutomationTagEvents` (wraps the tags event port → `tag_applied`),
  contract tests. `Organize::tags()/filing()` accessors added.
- **UI**: `features/automations/` (rail, builder with condition groups + step list + WIP picker, Test on latest,
  Duplicate, Delete, Matching meetings tab with Run on selected, "Allow shell scripts" switch), meeting header ⋯ →
  Automations submenu (`MeetingAutomationsMenu`), route `/automations?automation=<id|new>`, nav entry live,
  dev:mock (`dev/mock-ipc/automations.ts`, `mock-automations.ts`). The condition editor moved to the shared
  `features/conditions/` (insights uses it with its 6 kinds); `OptionSelect` moved to `ui/option-select.tsx`.
- §7.2 rows added (ORG-07 bulk entry not on main; DEV-01/INT steps; folder page link).

## Left for B
- Playwright e2e over dev:mock (create → add steps → save → matching → run; meeting ⋯ menu; WIP entries disabled).
- Visual polish, light/dark screenshots: header actions wrap badly at narrow widths; rail is empty while creating;
  step picker items use `title=` (their accessible name becomes the description: use a tooltip instead);
  export folder is a text input (offer `FolderDialog`/a picker); run log (`RunLog` in `run-status.tsx`) is not shown yet
  in the builder (show latest runs per meeting there).
- ADR 0042 (automations: runs table + Step registry + shell hook), `docs/ARCHITECTURE.md` (stage 95, plugin, tables),
  crate README for deino-core automations, §7.1 progress row, `docs/setup/` shell-hook payload doc.
- Full gate: `scripts/ci-local.sh`.

## Gotchas
- `.sqlx/` was regenerated with `scripts/sqlx-prepare.sh`; rerun it after any query change.
- Sidecar placeholders must exist to build the app crate (`frontend/src-tauri/binaries/*`, ignored by git).
- Tag trigger: ready/failed meetings → `pending` + stage scheduled; scheduled/recording/processing → `armed`, picked
  up by the stage; armed runs whose conditions stop matching are discarded. Existing runs are never replaced by
  triggers, only by manual Run/Rerun.
- Manual runs ignore conditions and the on/auto switches (Circleback behaviour).
- `Trigger` stored as `trigger` + `trigger_tag_id`; a deleted tag reads back as `manual`.

## Verify
```bash
export CARGO_BUILD_JOBS=6 RUSTC_WRAPPER=sccache
cargo test -p deino-domain conditions
cargo test -p deino-core --lib automations::
cargo test -p deino-pipeline stages::automations registry
cargo test -p deino --lib features::automations bindings
cargo clippy -p deino-domain -p deino-db -p deino-core -p deino-pipeline -p deino --all-targets -- -D warnings
scripts/check-rust-file-size.sh
cd frontend && pnpm typecheck && npx vitest run src/features/automations src/features/conditions src/features/insights
```
