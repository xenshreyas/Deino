# Review standards

What reviewers (people and agents) check on every Deino PR, and how severe a
finding is. PARITY_PLAN §3 and §3A are the rules; this list is where reviews
should look hardest. It is not exhaustive: flag any real correctness, privacy,
data, performance or lifecycle risk. Structure adapted from muesli's
`REVIEW.md` (MIT).

## Severity

- **P0:** data loss or corruption, a privacy leak (meeting content, prompts or
  secrets in logs, or sent anywhere the user did not connect), a crash on
  launch, a migration that edits history or cannot run on an existing
  database. Blocks merge.
- **P1:** a user-visible regression, a broken recording or transcription
  lifecycle, unbounded memory or CPU growth, blocking work on an audio
  callback or the UI thread, a §3A hard limit broken in new code, a lint
  suppression without a reason. Fix before merge.
- **P2:** missing tests for changed behaviour, an edge case without a
  resilient path, a layering shortcut (SQL in a service, logic in a command),
  undocumented public items. Fix or file a follow-up.
- **P3:** naming, comments, small cleanups.

A product decision recorded in PARITY_PLAN or an ADR is not a defect; argue it
in a new ADR, not in review.

## Checklist

**Privacy**
- No transcript text, notes, titles, prompts, emails or secrets in any log
  line, error message or event that leaves the process. Log IDs and counts.
- Data leaves the Mac only to services the user connected (§1, D2, FND-12).

**Data**
- New migrations only; never edit an applied file. File names use the real
  UTC time. New tables are `STRICT`, `NOT NULL` by default, with `CHECK`
  constraints and indexes for every list order and foreign key.
- Multi-row writes run in one transaction on the writer connection.
- A denormalised column names its source of truth and the one place that
  keeps it in sync.
- A new table has a `Table` variant, so `DbChanged` covers it.

**Layering**
- `commands.rs` only parses, calls one service method and maps the error.
- Services take ports (`Arc<dyn Trait>`) and a `Clock`; no SQL, no Tauri, no
  `Utc::now()`, no raw `tokio::spawn`, no `reqwest`.
- Every module error implements `ErrorClass`, and its messages are user-safe.
- Nothing under `crates/` depends on `tauri`.

**IPC**
- New commands live in a feature plugin, with the manifest, capability,
  `features::builders()` and bindings updated; nothing is added to `legacy.rs`.
- The UI calls generated bindings through a feature `api.ts` and TanStack
  Query hooks that declare `meta.tables`. No hand-written `invoke`.

**Quality**
- §3A limits hold in new code: 40-line functions, 300-line files, 4
  parameters, nesting depth 3, cognitive complexity 15. `scripts/clippy.sh`,
  `pnpm lint` and `pnpm typecheck` pass with no new exemptions.
- Legacy lists (`legacy_lints.rs`, `scripts/legacy-rust-files.txt`,
  `frontend/legacy-files.mjs`) only shrink.
- Public items have docs; comments explain why.
- `docs/ARCHITECTURE.md` and any ADR are updated in the same PR.

**Lifecycle**
- Spawned tasks are owned, tracked or explicitly detached with a logged error.
- Audio callbacks do not allocate, lock or log (§3C).
- Subscriptions are torn down when their owner goes away.

## Calibration notes

- `excessive-nesting-threshold = 5` in `clippy.toml` counts the `impl` and the
  function body, so it is §3A's "depth 3 inside a function" for a method;
  a free function gets one level more.
- `rustfmt` cannot see modules declared through `legacy_modules!`, so legacy
  files keep their inherited formatting until a card ports them; ported files
  are formatted in the same PR.
