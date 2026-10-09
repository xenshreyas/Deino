# Deino: Feature Parity Plan (vs Granola and Circleback)

| | |
|---|---|
| **Version** | v1, 2026-10-08 |
| **Baseline** | Deino = Meetily Community Edition v0.4.1 (commit `aeb1d24`) |
| **Targets** | Granola and Circleback, every feature as of Oct 2026 |
| **Research** | `docs/research/granola.md`, `circleback.md`, `community.md`, `meetily_audit.md` |

This document does two jobs:

1. It says clearly what is **left to build** in Deino before it has every feature Granola and Circleback have (§5 Gap matrix).
2. It splits that work into **independent task cards** that a sub-agent can pick up and finish alone (§6), with an execution order (§7).

---

## 0. How to use this document

**Orchestrator (main agent)**
1. Read §1–§4 once.
2. Take the next wave from §7. A card is ready when everything in its `Depends on` is merged to `main`.
3. Spawn one sub-agent per ready card, each in its own git worktree, on a branch named `feat/<CARD-ID>-<slug>`. The prompt is: *"Implement card `<CARD-ID>` from `docs/PARITY_PLAN.md`. Read §2–§4 and your card first. Follow §3 rules."*
4. When an agent finishes, check its acceptance checklist, merge to `main`, tick the card in §7, and start any cards that are now unblocked.
5. Two cards that list the same file under **Owns** must not run at the same time. §7 already schedules around this.

**Implementing sub-agent**
- Your card is your spec. Do what it says, and do not grow scope into another card. If you find a gap another card should fill, note it in your PR description under "Follow-ups".
- Every card's **Parity** line points at sections of `docs/research/*.md`. Read them for exact UX details such as wording, limits and edge cases.
- **Done** means: every acceptance box passes, `cargo nextest run --workspace` and `pnpm test` (Vitest) are green, `pnpm build` succeeds, and the PR description lists what you verified and how.

---

## 1. Product definition

**Deino is a private, local-first AI meeting notepad for one person.** It combines Granola's way of working (a calm notepad, no bot, your own notes enhanced by AI, chat across meetings) with Circleback's follow-through (stored recordings, speaker names, action items, insights, automations, integrations, API, MCP and CLI). Everything runs on the user's Mac by default. Cloud LLMs and integrations are opt-in, per provider.

**Fixed constraints (from the owner)**
- **English only.** Remove language pickers, translation and auto-detect. Hard-code `en` everywhere.
- **Transcription model quality is out of scope.** The default engine is **Parakeet Unified EN 0.6B** (`nvidia/parakeet-unified-en-0.6b`, GGUF via `transcribe-cpp`, see CAP-01). Do not spend effort on model accuracy beyond wiring that engine in.
- **A Mac app with no cloud services.** The app, recording, **transcription (always local)**, storage and search all run on the user's Mac. There is never a Deino server or hosted backend.
- **What may leave the Mac, and how.** Data goes **directly from the app** to services the user has connected, and nowhere else:
  - the LLM provider chosen for notes and chat (**Claude API or OpenAI API**), which receives transcript text and notes;
  - integrations the user connects (Slack, Notion, Gmail, calendars, CRMs, task tools).
  - Audio and transcription never leave the Mac.
- **No meeting bot.** Recording only ever happens on the computer running Deino. There are no share links or cloud sync (§5.8). A phone app is a future consideration (§10).
- **Proprietary.** Do not add open-source licence headers. Keep `THIRD_PARTY_NOTICES.md` up to date whenever code is copied from MIT projects such as Meetily, Handy or community PRs.

**Defaults unless §8 says otherwise**
- **Platform:** macOS 14.2+ on Apple Silicon is the primary target. Windows and Linux must still compile (`cargo check`), but new OS-specific features may be stubbed with `#[cfg(not(target_os = "macos"))]` returning `Err("unsupported on this platform")`.
- **Users:** a single user on a single machine. Team, workspace and admin features are not planned (§5.8).
- **Data location:** everything stays local under the app data dir.
- **Distribution:** the owner may hand the app to other people. Each copy is fully independent and keeps its own data on its own computer.

**Where Deino should be *better* than both products**
Each of these comes from a top complaint in `community.md` §6.2:
- audio is kept and can be played back (Granola has no audio);
- full history, with no 30-day limit;
- the transcript is editable;
- action items have due dates and two-way task sync;
- folders and tags are both available;
- people and companies can be added and edited by hand;
- no training or telemetry;
- a visible recording health indicator to catch silent failures.

---

## 2. Baseline: what Deino has today

Full detail is in `docs/research/meetily_audit.md`. Key facts:

**Architecture**
- **Stack:** a single Tauri 2 app.
  - The Rust core is in `frontend/src-tauri/src/`.
  - The UI is Next.js 14, statically exported, in `frontend/src/`.
  - It has a `llama-helper` sidecar (llama.cpp) and an `ffmpeg` sidecar.
  - **The Python `backend/` is legacy and unused.**
- **Database:** SQLite through sqlx. The file is `<app_data_dir>/meeting_minutes.sqlite` and migrations live in `frontend/src-tauri/migrations/`.
  - Tables: `meetings`, `transcripts` (has an unused `speaker` column), `summary_processes`, `transcript_chunks`, `settings` (API keys stored **in plaintext**), `transcript_settings`, `licensing` (unused), and `meeting_notes` (unused).
- **Saving:** the meeting row and transcript are written to SQLite **only when recording stops, and by the frontend** (`hooks/useRecordingStop.ts` → `api_save_transcript`). Live transcript text sits in React state and IndexedDB.
- **Audio:**
  - Mic comes from cpal. System audio comes from a Core Audio process tap on macOS (`audio/capture/core_audio.rs`).
  - **Mic and system are mixed before VAD/STT** (`audio/pipeline.rs:851-900`), so there is no Me/Them split.
  - Output is `audio.mp4` per meeting folder, with checkpoints.
- **Transcription:** a `TranscriptionProvider` trait in `audio/transcription/{engine,provider}.rs`. Engines: Parakeet TDT v2/v3 (ONNX via `ort`) and Whisper (whisper-rs).
- **Summaries:** `summary/{llm_client,processor,service}.rs`.
  - Providers: OpenAI, Claude, Groq, Ollama, OpenRouter, BuiltInAI (llama-helper) and CustomOpenAI.
  - Processing is chunk → combine → final.
  - 6 JSON templates in `frontend/src-tauri/templates/`, with custom JSON files read from `~/Library/Application Support/Meetily/templates/`. There is no template editor.
- **Commands:** about 190, all registered in **one** `tauri::generate_handler![]` at `src/lib.rs:612`.
- **Frontend:**
  - Routes: `/` (record), `/meeting-details?id=`, `/settings`, and `/notes/[id]` (a dead demo).
  - The sidebar is in `components/Sidebar/index.tsx`.
  - State lives in Context providers in `contexts/`.
  - The summary editor is BlockNote (`components/AISummary/`, `components/BlockNoteEditor/`).
- **Tests:**
  - Rust has about 250 tests, mostly in `summary/` and `whisper_engine/`, with none for the DB, API or audio.
  - The frontend has 10 `bun:test` files in `frontend/tests/` but **no `test` script**.
  - **CI runs no tests.**

**Features that work today (keep them working)**
- live transcript
- pause/resume
- crash recovery
- audio import and re-transcription
- template-based summaries, which can be edited, regenerated and cancelled
- auto-summary on stop
- onboarding
- tray
- notifications
- device hot-swap
- 7 LLM providers
- the built-in local LLM

**Partial or missing:** see §5.

---

## 3. Engineering rules for every agent

1. **Branch and worktree.** Use one card per branch, `feat/<CARD-ID>-<slug>`. Never push to `main` directly. Never push to the `upstream` remote, which is disabled anyway.
2. **Migrations.**
   - Name new files `crates/deino-db/migrations/<YYYYMMDDHHMMSS>_<card_id_lowercase>_<desc>.sql` (moved there from `frontend/src-tauri/migrations/` by FND-13), using the real current UTC time so two agents never produce the same name. Add a `Table` variant for each new table and run `scripts/sqlx-prepare.sh`.
   - Only change tables your card **owns** (see §4.1). Adding nullable columns to `meetings` is allowed for any card, but list them in your PR.
   - Never edit an existing migration.
3. **Where code goes (§4.0 is authoritative).**
   - **Feature logic** (service, repo, model, error) goes in `crates/deino-core/src/<feature>.rs` + `crates/deino-core/src/<feature>/`, which is Tauri-free.
   - **IPC** goes in **one inline Tauri plugin per feature**: `frontend/src-tauri/src/features/<feature>.rs` + `features/<feature>/{commands.rs, events.rs}`, with its own `tauri_specta::Builder`. Register it with one line each in `features/manifest.rs`, `lib.rs` (`.plugin(..)`) and `capabilities/default.json` (`"<feature>:default"`).
   - **No new `mod.rs` files.** Use the `foo.rs` + `foo/` style.
   - **Legacy Meetily commands** stay in `src/legacy/` under one `generate_handler!` until a card ports them.
4. **Frontend structure.**
   - New pages are TanStack Router file routes under `frontend/src/routes/` (thin; they render a feature screen).
   - Feature components go under `frontend/src/features/<feature>/`, which FND-10 creates.
   - Add sidebar entries **only** through the nav registry (`frontend/src/features/shell/nav.ts`, also from FND-10).
   - **Never hand-write `invoke()`.** Call the **generated** tauri-specta bindings in `frontend/src/lib/ipc/generated/` through TanStack Query hooks in `features/<feature>/queries.ts`.
   - **No raw `useEffect`.** Derive state, or use queries/mutations, event handlers or `useMountEffect` (anarlog rule).
5. **Settings.** New settings go in the typed settings store from FND-06. Never use `localStorage` for anything durable, and never write new plaintext secrets: **secrets go in the macOS Keychain** through FND-06.
6. **LLM calls.** Every LLM call goes through the LLM gateway (FND-09) and names a **task route**, such as `notes.enhance`, `chat`, `extract.action_items` or `tagging`. Never call a provider directly.
7. **Background work.** Anything that runs after a meeting is a **pipeline stage** (FND-08). Never chain work with ad-hoc `tokio::spawn`.
8. **Tests are required.**
   - Rust: write unit tests for pure logic and repository tests against `deino_db::test_db()` (in-memory SQLite, all migrations applied). Run them with `cargo nextest`.
   - Frontend: write **Vitest** tests (Testing Library, happy-dom) colocated as `*.test.ts(x)`, plus Playwright smoke tests over `pnpm dev:mock`.
   - Any LLM-dependent logic must be testable with the gateway's `MockProvider` (FND-09).
9. **UI verification.** Run `pnpm dev:mock` (FND-04), which runs the Vite app in a browser with Tauri IPC mocked from fixture data. Screenshot your UI in the browser pane and attach what you saw to the PR. Real-hardware checks of audio, permissions or AX go in a "Manual QA" checklist in the PR.
10. **Dependencies.**
    - Prefer crates and packages that are already in the tree.
    - If you add one, pin its version, say why in the PR, and keep its licence permissive: MIT, Apache-2.0, BSD or ISC. **No GPL or AGPL.**
    - Never run downloaded binaries during the build without a sha256 pin.
11. **Platform guards.** macOS-only code goes behind `#[cfg(target_os = "macos")]` with a stub for other platforms, and `cargo check` must pass on Linux in CI.
12. **Keep it single-user.** Do not invent auth, users or tenants.
13. **Code quality.** Follow §3A for all new code. Remove dead code you replace. When you touch a legacy file that breaks §3A, improve the part you touch, but don't rewrite unrelated code.
14. **Benchmark against the best open-source projects.**
    - Local clones live in `~/Documents/GitHub/deino-references/`. **`docs/REFERENCES.md` maps each card to the exact reference files to study first.** Read them before designing, and match or beat their code quality and performance.
    - **Copyable** (MIT/Apache; port with an entry in `THIRD_PARTY_NOTICES.md`): Handy (owner's daily-driver STT, Parakeet), Anarlog (ex-Hyprnote; closest product analog), Spacedrive, Muesli, OpenWhispr, sherpa-onnx, tauri plugins-workspace, tauri-specta, shadcn/ui. Meetily community PRs are also MIT.
    - **Reference-only** (copyleft; learn the patterns, never copy code): Cap and Zed.
    - Do **not** copy from Granola, Circleback or Meetily PRO.
    - In the PR description, name the reference you benchmarked against and how Deino compares (structure, latency, CPU, memory).

---

## 3A. Code quality standards

Owner goal: **extremely high-quality, beautiful code. Nothing less is acceptable.** It must be readable, maintainable, well documented and easy to test, so later features and bug fixes are cheap. Reviewers reject PRs that break these rules. FND-13 sets up the tooling, lint enforcement and a reference module.

### Hard limits (enforced by lint where possible)
- **Functions: 40 lines maximum** (clippy `too_many_lines` threshold 40; ESLint `max-lines-per-function` 40, which excludes JSX-only render bodies of presentational components when those are split into sub-components).
- **Files: 300 lines maximum**, tests excluded (ESLint `max-lines`, plus a CI script for Rust). This is a hard cap for new code. Legacy files over the cap get split when a card substantially changes them.
- **Function arguments: 4 maximum.** Use a typed params struct or object beyond that.
- **Nesting depth: 3 maximum.** Prefer early returns.
- **Cognitive complexity:** clippy `cognitive_complexity` 15, ESLint `sonarjs/cognitive-complexity` 15.

### Design principles
- **SOLID:**
  - **Single responsibility:** each module, type and function does one thing you can name in a short phrase.
  - **Open/closed:** extend through registries and traits (pipeline stages, LLM providers, integrations, MCP tools, note templates) without editing the core.
  - **Liskov:** every implementation of a trait honours its documented contract, and contract tests run against every implementation.
  - **Interface segregation:** keep traits small (`TranscriptReader` vs `TranscriptWriter`, not one `TranscriptStore` god-trait).
  - **Dependency inversion:** services depend on traits, and concrete adapters are wired up in one composition root (`app_state.rs`).
- **Object-oriented design where it fits.** Rust has no inheritance, so model "is-a" with **traits, with default methods for shared behaviour** and composition for "has-a". Use the newtype pattern for domain IDs (`MeetingId(Uuid)`, `PersonId`, …) so IDs can never be mixed up. Use enums and state machines for lifecycle (`MeetingStatus`, `RecordingState`) with explicit transition methods. In TypeScript, use classes only where they model a real object with behaviour (a player controller, for example). Otherwise use functions, hooks and discriminated unions.
- **Names** are clean, short and descriptive: `enhance_notes`, `MeetingRepo`, `useMeeting`. No abbreviations except well-known ones (id, url, db, llm, ui). No `data`, `info`, `manager`, `helper` or `utils` dumping grounds.
- **DRY, but no premature abstraction.** Abstract on the third use.

### Typing (zero tolerance)
- **TypeScript:**
  - `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`.
  - **No `any`, no `as` casts except at validated boundaries, no `object`/`{}`/`Record<string, unknown>` as a lazy type, and no `// @ts-ignore`.** ESLint `no-explicit-any` and `no-unsafe-*` are errors.
  - Use discriminated unions for variants and branded types for IDs.
  - External data (IPC, JSON files) is validated with **zod** at the boundary when it isn't covered by generated types.
- **Rust:**
  - Newtypes for IDs and units (`Millis(u64)`, `Seconds`).
  - Enums instead of stringly-typed fields. Every DB TEXT enum column maps to a Rust enum via `sqlx::Type`.
  - No `serde_json::Value` passed around inside the core. Parse it into a typed struct at the edge.
  - No `String` errors inside the core.

### Database design
- **Normalize to 3NF by default:** entities in their own tables, many-to-many through join tables (`meeting_attendees`, `meeting_tags`, `meeting_folders`), and no repeated groups in JSON columns when they need to be queried.
- **Denormalize deliberately, and document it:**
  - Cached aggregates the UI lists need fast, for example `meetings.duration_s`, `meetings.attendee_count`, `people.last_met_at`, `people.meeting_count`.
  - The FTS5 and vector index tables.
  - Write-once snapshots, such as an `enhanced_notes` version.
  - Every denormalized column has a comment in its migration naming its **source of truth** and the **one place** that keeps it in sync: a repo method in the same transaction, or a trigger.
- **JSON columns** are allowed only for opaque payloads that are never queried by field (the TipTap document JSON, raw provider payloads). Each has a typed Rust struct.
- **Integrity and performance:**
  - Foreign keys ON (`PRAGMA foreign_keys=ON`) with explicit `ON DELETE` behaviour.
  - `CHECK` constraints for enums.
  - `NOT NULL` by default.
  - Indexes for every FK and every filter or sort used by a list screen. Check with `EXPLAIN QUERY PLAN` in a test for the hot queries.
  - Multi-row writes happen in transactions.
  - WAL mode, with one write connection and a read pool.
  - Queries use the sqlx `query_as!` macros (compile-time checked) with the offline `.sqlx` data committed.

### Structure (Rust)
- **Layers.** Each feature module has these layers. Each layer may only call the one below it.
  1. `commands.rs` (in the app crate's feature plugin): thin Tauri commands. They parse input, call one service function and map errors. No logic.
  2. `service.rs`: business logic. It has no Tauri types and no SQL, and it is pure where it can be.
  3. `repo.rs`: SQL only, through sqlx. One function per query, returning typed structs.
  4. `adapters/`: anything external (LLM, HTTP, OS APIs, audio devices, filesystem, clock), each **behind a trait**.
- **Dependency injection.** Services receive their dependencies as trait objects or generics: `LlmGateway`, `Clock`, `HttpClient`, `CalendarSource` and so on. Tests pass fakes. Never reach for globals or singletons in new code.
- **Errors.**
  - Each module has a `thiserror` enum describing what can go wrong.
  - Commands convert errors to the shared serializable `AppError { code, message }` (FND-13), so the UI can react to `code`.
  - `anyhow` is allowed only at the edges, such as `main` and one-off scripts.
  - No `unwrap`/`expect` outside tests unless it guards a real invariant, with a comment saying why it can't fail.
- **Size.** See the hard limits above. Split by responsibility, not arbitrarily.
- **Constants.** No magic numbers or strings: use named `const`s with a doc comment (for example `const AUTO_STOP_SILENCE: Duration = …`).
- **Logging.** Use the `log` crate with the module as target. **Never log transcript text, notes, prompts, emails or secrets**; log IDs and counts instead.

### Structure (frontend)
- **TypeScript.** `strict` mode, no `any`, and no non-null `!` without a comment.
- **Types shared with Rust are generated, not hand-written.** Use `tauri-specta` (or `ts-rs`), so the IPC types can't drift (FND-13).
- **Feature folders.** `features/<name>/` contains `components/` (presentational, props in, events out), `hooks/` (data and state), `api.ts` (typed IPC calls) and `types.ts`.
- **Server state.** Data that comes from Rust goes through **TanStack Query** hooks over IPC: caching, invalidation on Rust events, and loading and error states. UI-only state stays local, or in a small Zustand store when shared.
- **UI.** Build with shadcn/Radix components and Tailwind design tokens (see §3B). Keep visible strings in one `copy.ts` per feature.

### Documentation
- **Rust.** Every module starts with a `//!` doc covering what it does, how it fits in and its main types. Every `pub` item has `///` docs.
- **TypeScript.** Exported functions, hooks and components have TSDoc.
- **Comments** explain *why* (constraints, edge cases, links to research sections), not *what*.
- **`docs/ARCHITECTURE.md`** (FND-13) is the map of the system:
  - processes
  - modules
  - data flow (record → pipeline → notes)
  - DB schema diagram
  - extension points (new pipeline stage, LLM route, integration, MCP tool)

  **Update it in the same PR** whenever a card changes any of these.
- **ADRs:** a significant design choice (a library, a data model, an algorithm) gets a short ADR in `docs/adr/NNNN-title.md` (context, decision, alternatives, consequences).
- **Setup docs:** every integration documents its setup in `docs/setup/<provider>.md`.

### Testing
- **Unit tests** for every service function, using fakes for adapters and `Clock` for anything time-based.
- **Repository tests** against an in-memory SQLite with all migrations applied (`test_db()`).
- **Contract tests** for every Tauri command: input → output JSON shape, so the frontend can rely on it.
- **Frontend:** component and hook tests with Vitest, Testing Library and happy-dom. Use Playwright smoke tests for main flows against `pnpm dev:mock`.
- **LLM features:** prompt-snapshot tests with `MockProvider`, plus a small **eval set** (fixtures and expected facts) runnable on demand with a real key (`pnpm eval <feature>`). Evals are not part of CI.
- **Coverage:** new service and repository code should reach about 80% line coverage. CI reports it (`cargo llvm-cov`, Vitest coverage).
- **Bug fixes** start with a failing test that reproduces the bug.

### Tooling enforced in CI (FND-03 and FND-13)
- **Rust:** `rustfmt`, `clippy -D warnings` with a curated set of pedantic lints in `clippy.toml`.
- **Frontend:** ESLint (typescript-eslint strict, react-hooks) and Prettier. `tsc --noEmit`.
- **Tests:** all tests must pass.
- **Commits:** Conventional Commits.
- **PR template:** what, why, how it was tested, screenshots, docs updated, follow-ups.

### Definition of done (every card)
- [ ] The acceptance checklist passes.
- [ ] Tests were added at the right layers, and CI is green.
- [ ] Module docs and public-item docs are written, and `ARCHITECTURE.md` and any ADR are updated.
- [ ] No new lint suppressions without a comment.
- [ ] No secrets or meeting content in logs.

---

## 3B. UI design direction

**Benchmarks:** Granola, Circleback, Gumloop (gumloop.com), Linear, Notion and Things. The UI must be **very clean, minimal, simple, modern and elegant**.
- **Themes:** Light and dark, **following the system by default** and configurable in Settings (System / Light / Dark). Both themes are designed equally, not "dark = inverted".
- **Layout:**
  - Lots of whitespace and a content-first layout.
  - A slim sidebar.
  - The notepad reads like a document: a max width of about 720 px, generous line height.
  - No cards inside cards, no heavy borders, no gradients except the subtle accent of the recording state.
- **Typography:**
  - One family: Inter, or the SF system stack.
  - A 4-step type scale (12/14/16/24) and 2 weights (400/600).
  - Muted grey for secondary text, and AI-generated text grey vs user text in the primary colour (NOT-02).
- **Colour:**
  - Neutral greys (warm-neutral, like Granola) plus **one** accent (for example a calm green for recording or brand).
  - Semantic tokens only: `--bg`, `--bg-subtle`, `--fg`, `--fg-muted`, `--border`, `--accent`, `--danger`, … **No hard-coded colours in components.**
- **Components:**
  - shadcn/Radix restyled to the tokens, with 8 px radius and hairline 1 px borders.
  - Soft shadows only on floating layers (popovers, the record panel).
  - Lucide icons at 16 px with a 1.5 stroke.
- **Motion:** 120–200 ms ease-out on hover, open and close; respect `prefers-reduced-motion`; no bouncy animations.
- **Interaction:**
  - Keyboard-first: ⌘K, ⌘J, ⌘N, and shortcuts shown in tooltips.
  - Every list has an empty state with one clear action.
  - Loading uses skeletons, not spinners.
  - Errors are inline and human-readable.
- **Recording UI:**
  - Calm and unobtrusive: the floating panel is a small pill with a level meter.
  - Never a modal during a meeting.
- **Design system:** FND-10 builds the tokens, primitives and a `/design` route (dev-only) that shows every primitive in both themes. Every UI card follows it and attaches light and dark screenshots to its PR.

## 3C. macOS audio and system integration rules

- **System audio:** use the **Core Audio process tap** (`AudioHardwareCreateProcessTap` + aggregate device, macOS 14.2+), as the current code does. No virtual drivers (BlackHole), no kernel extensions. Keep ScreenCaptureKit audio as a fallback backend only.
- **Microphone:** use cpal/Core Audio with the user's chosen device or the system default.
  - Follow default-device changes (`kAudioHardwarePropertyDefaultInputDevice` listener).
  - Handle Bluetooth headsets: warn when HFP lowers quality, and prefer the built-in mic when headphones are in use (Granola's tip).
  - Never request exclusive access.
- **Per-process detection:** use the Core Audio process objects (`kAudioHardwarePropertyProcessObjectList`, `kAudioProcessPropertyIsRunningInput`, `kAudioProcessPropertyBundleID`), not polling `ps`.
- **Real-time safety:** audio callbacks never allocate, lock or log. Copy frames into a lock-free ring buffer (`rtrb` or the existing ring buffer) and process on a worker thread.
- **Format:** capture at the device's native rate and resample once (`rubato`) to 16 kHz mono for STT and 48 kHz for storage. Keep mic and system **as separate tracks** end to end (CAP-02).
- **Lifecycle:**
  - Handle sleep and wake (`NSWorkspace` notifications), device disconnects, sample-rate changes and the tap being invalidated, with automatic recovery and a health event (CAP-10).
  - Tear down the tap and aggregate device cleanly on stop and crash, with no leaked aggregate devices (clean up stale `deino-audio-tap*` devices on launch).
- **Permissions:** request mic, audio capture (`NSAudioCaptureUsageDescription`), screen recording only if CAP-09 is enabled, and Accessibility, Calendars and Reminders only when the feature needing them is first turned on. Every permission has a clear explainer screen and a deep link to the right System Settings pane.
- **Native APIs from Rust:** use the `objc2` family of crates (`objc2-core-audio`, `objc2-event-kit`, `objc2-vision`, `objc2-app-kit`) or the existing `cidre`. No shelling out to AppleScript except for the Apple Notes export.

---

## 4. Target architecture (shared foundations)

### 4.0 Repository architecture (authoritative; decided 2026-10-08 from `docs/references/architecture.md`)

**If a card's file paths conflict with this section, this section wins.** Cards written earlier mention paths like `frontend/src-tauri/src/<feature>/` or `pipeline/`; map them to the layout below.

```
Deino/
├── Cargo.toml                 # workspace; [workspace.dependencies]; [workspace.lints]; dev deps opt-level=1
├── rust-toolchain.toml  clippy.toml  rustfmt.toml  deny.toml  typos.toml  lefthook.yml  cliff.toml
├── .cargo/config.toml         # SQLX_OFFLINE=true, MACOSX_DEPLOYMENT_TARGET=14.2
├── .sqlx/                     # committed sqlx offline query data
├── crates/                    # NONE of these depend on tauri
│   ├── deino-domain/          # newtype IDs, enums (sqlx::Type + specta::Type), DTOs, AppError/ErrorCode, Clock
│   ├── deino-db/              # pool (WAL, 1 writer + readers), migrations/, MIGRATOR, change notifier, test_db()
│   ├── deino-core/            # features as modules: meetings/, people/, folders/, tags/, action_items/, settings/, …
│   ├── deino-pipeline/        # Runner, PipelineStage trait, registry.rs, job repo
│   ├── deino-llm/             # gateway, routes, providers, structured output, tools, askama/minijinja prompts, mock, evals
│   ├── deino-net/             # the ONLY reqwest owner: ServiceId registry, ScopedClient, egress log, offline mode
│   ├── deino-secrets/         # SecretStore trait, Keychain + in-memory impls
│   ├── deino-audio/           # capture (cpal, Core Audio tap via cidre), rings, tracks, AudioSource trait, mock source
│   ├── deino-stt/             # transcribe-cpp worker supervisor, engines; MUST NOT depend on deino-net
│   ├── deino-doc/             # TipTap JSON <-> Markdown (adapted from anarlog crates/tiptap, MIT)
│   └── deino-testkit/         # fixture loaders + fakes (dev-dependency only)
├── llama-helper/              # local LLM sidecar (separate process)
├── fixtures/meetings/*.json  fixtures/audio/<meeting>/{mic,system}.m4a
└── frontend/
    ├── vite.config.ts  vitest.config.ts  playwright.config.ts  eslint.config.js  tsconfig.json
    ├── src-tauri/
    │   ├── build.rs                     # tauri_build + InlinedPlugin per features/manifest.rs
    │   ├── capabilities/default.json
    │   └── src/  main.rs  lib.rs (≤150 lines)  composition.rs (DI root)  bindings.rs (export test)
    │             features.rs  features/manifest.rs  features/<feature>.rs  features/<feature>/{commands,events}.rs
    │             legacy/ (Meetily commands until ported)
    └── src/
        ├── main.tsx  router.tsx  routes/ (TanStack Router, file-based, thin)
        ├── features/<feature>/{components/, hooks/, api.ts, queries.ts, copy.ts, types.ts, index.ts}
        ├── features/shell/{nav.ts, command-palette/, sidebar/}   features/meeting/slots.ts
        ├── lib/ipc/{generated/, result.ts, events.ts, query-client.ts}
        ├── ui/ (shadcn primitives on tokens; icons.ts)   styles/tokens.css
        └── dev/mock-ipc/<feature>.ts    # `satisfies typeof generated.<feature>.commands`
```

**Dependency rules**
- `domain ← db ← core ← pipeline ← app`. `llm → net`. `stt ↛ net`, checked in CI with `cargo tree`.
- No crate under `crates/` depends on `tauri`.
- A feature becomes its own crate only when it gains heavy native deps or grows past about 3k lines.

**Frontend stack**
- Vite, React 18, TanStack Router (file-based, typed params, zod search params), TanStack Query (Rust data, invalidated by `DbChanged` events from SQLite commit hooks), Zustand (small shared UI state).
- Tailwind v4 with semantic tokens (`ui.md`), shadcn/Radix, lucide icons through one `ui/icons.ts`.
- Editor: **TipTap 3**, replacing BlockNote from NOT-01 on. TipTap JSON is the stored document format, so Rust can convert it with `deino-doc`.
- Tests: Vitest and Playwright.

**IPC**
- tauri-specta (`=2.0.0-rc.21`, matching Handy) generates the TS bindings from a `#[test]`; CI fails if they are stale.
- All errors are `AppError { code, message }`.
- LLM token streams use a per-request typed `tauri::ipc::Channel`.

**Process isolation**
- transcribe-cpp runs in **child worker processes** (Handy's supervisor pattern).
- The local LLM runs in the llama-helper sidecar.
- A native crash or hang can't take the app down.

### 4.1 Data model: core tables and who owns them

FND-07 creates the core schema in one migration so later cards don't collide. Feature-specific tables are created by the card that owns them. All IDs are TEXT UUIDv7 unless noted, and all timestamps are TEXT ISO-8601 UTC.

| Table | Created by | Purpose / key columns |
|---|---|---|
| `meetings` (extend) | FND-07 | Adds `status` (`scheduled`/`recording`/`processing`/`ready`/`failed`), `started_at`, `ended_at`, `duration_s`, `calendar_event_id` FK, `series_id`, `template_id`, `source` (`live`/`import`/`api`), `deleted_at` (trash), `audio_path`, `audio_mic_path`, `audio_system_path`, `video_path` |
| `transcript_segments` | FND-07 | Replaces `transcripts` for new data. Columns: `id`, `meeting_id`, `channel` (`mic`/`system`/`mixed`), `speaker_id` FK, `start_ms`, `end_ms`, `text`, `words_json` (token timestamps), `is_deleted`, `edited_at`. A view or migration keeps old `transcripts` readable. |
| `speakers` | FND-07 | `id`, `meeting_id`, `label` ("Speaker 2"), `person_id` FK nullable, `is_me`, `color` |
| `people` | FND-07 | `id`, `name`, `email` UNIQUE nullable, `company_id`, `title`, `avatar_path`, `notes_md`, `summary_md`, `summary_updated_at`, `is_me`, `merged_into` |
| `companies` | FND-07 | `id`, `name`, `domain` UNIQUE, `logo_path`, `notes_md`, `summary_md` |
| `meeting_attendees` | FND-07 | `meeting_id`, `person_id`, `role` (`organizer`/`invitee`/`speaker_only`), `rsvp` |
| `calendar_accounts`, `calendar_events` | FND-07 (filled by ORG-01) | Event: `id`, `account_id`, `provider_event_id`, `ical_uid`, `series_id`, `title`, `description`, `start_at`, `end_at`, `conference_url`, `platform`, `organizer_email`, `attendees_json`, `my_rsvp`, `is_external`, `record_override` (null/true/false). No `meeting_id`: the link is `meetings.calendar_event_id` only (ADR 0012) |
| `folders`, `meeting_folders` | FND-07 | Folders are nested via `parent_id`, plus `name`, `icon`, `description`, `auto_rules_json`. A meeting can sit in many folders. |
| `tags`, `meeting_tags` | FND-07 | Tag: `name`, `color`, `description`, `auto_apply`. Link: `meeting_id`, `tag_id`, `applied_by` (`user`/`ai`/`automation`) |
| `action_items` | FND-07 | `id`, `meeting_id` nullable, `title`, `description`, `assignee_person_id`, `due_at`, `status` (`open`/`done`), `priority`, `source_segment_id`, `external_refs_json`, `completed_at`, `created_by` (`ai`/`user`) |
| `meeting_notes` | NOT-01 | User notes: `notes_json` (TipTap JSON, `deino-doc`), `notes_markdown` (derived, Readable) |
| `enhanced_notes` | NOT-02 | `meeting_id`, `version`, `blocks_json` (blocks carry provenance and citations), `markdown`, `template_id`, `model`, `created_at`, `is_user_edited` |
| `templates` | NOT-04 | Moves templates from JSON files into the DB; built-ins are seeded |
| `jobs`, `job_runs` | FND-08 | The pipeline queue |
| `chat_threads`, `chat_messages` | AI-02 | |
| `recipes` | AI-04 | |
| `embeddings` (sqlite-vec virtual table) and `search_fts` (FTS5) | AI-01 / ORG-08 | |
| `insight_defs`, `insight_results` | AI-06 | |
| `automations`, `automation_runs` | AUT-01 | |
| `integration_connections` (no secrets; tokens live in the Keychain) | INT-00 | |
| `webhooks`, `webhook_deliveries` | DEV-01 | |
| `api_keys` (hashed) | DEV-03 | |
| `voiceprints` | CAP-04 | `person_id`, `embedding BLOB`, `source_meeting_id`, `quality` |
| `clips` | TRN-04 | |
| `vocab_terms` | TRN-05 | |
| `snapshots` | CAP-09 | |

### 4.2 Post-meeting pipeline (FND-08)

A durable, SQLite-backed job runner. When a meeting changes to `processing`, the runner executes the registered stages in order. Each stage is idempotent, records its status in `job_runs`, can be re-run from the UI, and emits a `pipeline-progress` event.

| Order | Stage | Card |
|---|---|---|
| 10 | `finalize_audio` (merge checkpoints, write mic/system/mixed files) | FND-05 |
| 20 | `final_transcription` (optional higher-quality re-pass, or record-only mode) | CAP-08 |
| 30 | `diarize` | CAP-03 |
| 35 | `identify_speakers` (voiceprints and calendar names) | CAP-04 |
| 40 | `vocab_fixups` | TRN-05 |
| 50 | `enhance_notes` | NOT-02 |
| 55 | `screen_context` (OCR of snapshots) | CAP-09, which runs before 50 if enabled |
| 60 | `extract_action_items` | AI-05 |
| 65 | `auto_title` | NOT-02 |
| 70 | `auto_tag` | ORG-06 |
| 75 | `auto_folder` | ORG-05 |
| 80 | `index` (FTS and embeddings) | ORG-08 / AI-01 |
| 85 | `insights` | AI-06 |
| 90 | `update_entities` (people and company summaries) | ORG-04 |
| 95 | `automations` | AUT-01 |
| 98 | `follow_up_draft` | NOT-07 |
| 99 | `auto_export` | EXP-02 |

Stages register through a trait: `trait PipelineStage { fn id(&self)->&'static str; fn order(&self)->u16; async fn run(&self, ctx:&StageCtx, meeting: MeetingId)->Result<StageOutcome, StageError>; }` (`StageError::is_retryable()`). Adding a stage means one line in `crates/deino-pipeline/src/registry.rs`.

### 4.3 LLM gateway (FND-09)

- **Interface:** one Rust interface, `llm::Gateway`. It offers `complete(route, messages, opts)`, `stream(...)`, `structured::<T: JsonSchema>(...)` (JSON-schema output, validated with retry), and `with_tools(...)` (a tool-calling loop).
- **Providers:**
  - Anthropic (Claude) and OpenAI-compatible (OpenAI, Groq, OpenRouter, custom) support tool calling.
  - Ollama supports tool calling on models that have it.
  - BuiltIn (llama-helper) gains streaming and a GBNF grammar for structured output.
- **Task routes** (each user-configurable in Settings → AI, with a provider and model each):
  - `notes.enhance`
  - `notes.edit`
  - `chat`
  - `extract.action_items`
  - `extract.insights`
  - `tagging`
  - `title`
  - `entity.summary`
  - `brief`
  - `followup.email`
  - `embed`
- **Default routing (§8 D3):**
  - **Claude:** `claude-sonnet-5-5` for `notes.*`, `chat`, `brief` and `followup.email`; `claude-haiku-5-5` for `extract.*`, `tagging`, `title` and `entity.summary`.
  - **OpenAI:** the user picks a "smart" and a "fast" model from the provider's live model list.
  - **No key:** if no API key is set, routes fall back to the built-in local model.
  - `embed` is always local.
  - All provider traffic goes through the FND-12 network client.
- **Logging:** usage per call (tokens, latency, route) goes to a local `llm_calls` table for a usage page. Prompts are never logged unless the user turns on debug mode.
- **Testing:** `MockProvider` returns canned responses keyed by route, for tests.

### 4.4 Retrieval (ORG-08 and AI-01)

- **Keyword search:** FTS5 virtual table `search_fts(kind, ref_id, meeting_id, title, body, ts_ms)` covering meeting titles, enhanced notes, user notes, transcript segments, action items and people.
- **Semantic search:** `sqlite-vec` table `embeddings(ref_kind, ref_id, meeting_id, chunk_ix, vec float[N])`.
  - Local embedding model: `nomic-embed-text-v1.5` GGUF through llama-helper's new `Embed` request.
  - An optional cloud embed route exists.
  - Chunks are about 400 tokens, built from transcript windows (by speaker turn) and note sections.
- **Hybrid query:** `search(query, filters) -> Vec<Hit{kind, meeting_id, ref_id, ts_ms, snippet, score}>` uses reciprocal rank fusion. Chat, global search, MCP and API all use it.

### 4.5 Integrations framework (INT-00)

- **OAuth:** loopback OAuth 2.0 with PKCE (`http://127.0.0.1:<random>/callback`). Tokens go in the Keychain (`deino.<provider>.<account>`) and refresh automatically.
- **Connections UI:** Settings → Connections lists each connection with its status, last sync time and errors.
- **Client IDs:** each provider's client ID is read from the settings store, so the user can paste their own (see §9). There are no hard-coded secrets.
- **Destination trait:** each provider implements `trait Destination { capabilities(); async fn push_note(...); async fn push_action_items(...); async fn upsert_crm_activity(...) }`, and both automations and the "Share to…" menu use it.

### 4.6 Frontend shell (FND-10)

- **Layout:** a left sidebar with Home, Meetings, Action Items, People, Companies, Folders (tree), Tags, Insights, Recipes, Automations and Settings. The bottom of the sidebar holds record controls and the recording state.
- **Global UI:**
  - ⌘K command palette (search and navigation)
  - ⌘J chat
  - ⌘N new note
  - ⌘⇧R start/stop recording
  - `[` and `]` toggle the side panels
  - Light/dark/system theme
- **Meeting page:**
  - Header: title, date, attendees, tags, folders, share/export menu.
  - Tabs: Notes (Enhanced / My notes), Transcript, Insights, Clips.
  - Right panel: Chat, Participants, Details.
  - Bottom: audio player.

---

## 5. Gap matrix: what is left to build

Legend:
- **G** = Granola has it.
- **C** = Circleback has it.
- **Deino today:** ✅ done · 🟡 partial · ❌ missing.
- **Card** = the card in §6 that closes the gap.
- "—" means the product doesn't have the feature.

### 5.1 Capture

| Feature | G | C | Deino today | Card |
|---|---|---|---|---|
| Bot-free mic and system audio capture | ✅ | ✅ | ✅ | — |
| **Me vs Them split by channel** | ✅ | — | ❌ (mixed before STT) | CAP-02 |
| **Speaker diarization, "Speaker N"** (desktop and in-person) | iOS only | ✅ (including hybrid rooms) | ❌ | CAP-03 |
| **Voice profiles: rename once, recognized in future meetings** | — | ✅ | ❌ | CAP-04 |
| **Real names from the meeting app UI** (Zoom/Teams/Meet active speaker) | ✅ | ✅ (bot) | ❌ | CAP-05 |
| **Meeting detection when an app uses the mic** (per-app titles, ignore list) | ✅ | ✅ | ✅ (CAP-06, `deino-detect`) | CAP-06 |
| **Calendar notification 1 min before, which joins the call and starts recording** | ✅ | ✅ | ❌ | ORG-02 |
| **Auto-start (countdown) and auto-stop** (mic released, silence, sleep, calendar end) | 🟡 | ✅ | ❌ | CAP-06, CAP-07 |
| **Floating recording indicator and record panel** (pause/stop/mic, quick notes) | ✅ | ✅ | ❌ (tray only) | CAP-10 |
| **Recording health warning** (silent-failure detection) | ✅ | — | ❌ | CAP-10 |
| Pause/resume | ✅ | ✅ | ✅ | — |
| Switch mic or system audio mid-recording | ✅ | ✅ | 🟡 (auto hot-swap only) | CAP-10 |
| **Change the linked calendar event mid-recording** | ✅ | ✅ | ❌ | ORG-03 |
| **Screen/window video recording** | — | ✅ | ❌ | CAP-09 |
| **Shared-screen snapshots fed into notes** | ✅ | ✅ | ❌ | CAP-09 |
| **Recording-only mode** (transcribe afterwards) | — | ✅ (effectively) | ❌ | CAP-08 |
| Audio file import | — | ✅ (2 GB) | ✅ (beta flag) | CAP-11 (promote, add video, link to event) |
| **Folder watcher auto-import** | — | — | ❌ (OSS peers have it) | CAP-11 |
| **Import from other note-takers** (Granola, Circleback, Fathom, Fireflies, Read AI) | ✅ (account move) | ✅ | ❌ | CAP-12 |
| Phone and in-person mobile capture, Apple Watch | ✅ | ✅ | ❌ | **Future** (§10); in-person meetings are recorded on the Mac with CAP-03 |
| Meeting bot (Zoom/Meet/Teams/Webex) | — | ✅ | ❌ | **Won't build** (owner decision) |
| Slack huddles | ✅ (desktop audio) | ✅ (bot) | ✅ (desktop audio) | — |
| **Consent tools** (automatic chat message, watermark camera) | ✅ | 🟡 (bot message) | ❌ | CAP-13 |
| Device hot-swap, Bluetooth warning | ✅ | ✅ | ✅ | — |

### 5.2 Transcript

| Feature | G | C | Deino today | Card |
|---|---|---|---|---|
| Live transcript | ✅ | — | ✅ | — |
| **Chat-bubble layout** (Me right, Them left) and speaker colours | ✅ | ✅ | ❌ | TRN-01 |
| **In-transcript search (⌘F), including by speaker** | ✅ | ✅ | ❌ | TRN-01 |
| **Delete transcript chunks** (with a prompt to regenerate) | ✅ | — | ❌ | TRN-02 |
| **Edit transcript text, search and replace** | — | 🟡 (speaker names only) | ❌ | TRN-02 |
| **Rename a speaker for one segment or all segments** | ✅ (via notes) | ✅ | ❌ | CAP-04 |
| **Audio/video playback synced to the transcript** (click to seek, highlight, scroll to now, speed) | — | ✅ | ❌ (`AudioPlayer.tsx` is empty) | TRN-03 |
| **Clips and moments** (link to a timestamp) | — | ✅ | ❌ | TRN-04 |
| **Custom vocabulary / jargon** (and attendee names automatically) | ✅ | ✅ | ❌ | TRN-05 |
| **Comments on transcript ranges** | — | ✅ | ❌ | TRN-06 |
| Copy transcript | ✅ | ✅ | ✅ | — |
| Download audio | — | ✅ | 🟡 (open folder) | TRN-03 |

### 5.3 Notes

| Feature | G | C | Deino today | Card |
|---|---|---|---|---|
| **Notepad during the meeting** (markdown, checkboxes, images, @mentions) | ✅ | ✅ (private notes) | ❌ (table exists, no UI) | NOT-01 |
| **Notes before the meeting** (agenda on an upcoming event) | ✅ | ✅ | ❌ | NOT-01, ORG-02 |
| **AI-enhanced notes that keep the user's text** (user text black, AI text grey) | ✅ | 🟡 (private notes guide emphasis) | ❌ | NOT-02 |
| **Citations: each AI bullet links to its transcript evidence** | ✅ | — (assistant only) | ❌ | NOT-03 |
| Auto-generate when the meeting ends | ✅ | ✅ | ✅ | NOT-02 (rebuild on the new pipeline) |
| Regenerate, and switch template then regenerate | ✅ | — | ✅ / 🟡 | NOT-04 |
| **Warning before regenerating over your edits** | ✅ | — | ❌ | NOT-02 |
| **Template editor** (sections and per-section instructions, duplicating a built-in) | ✅ | — | ❌ (JSON files only) | NOT-04 |
| **About 30 built-in templates, default template, Auto choice by meeting type** | ✅ | — | 🟡 (6, no Auto) | NOT-04 |
| **`/template` in the notepad, before or during the meeting** | ✅ | — | ❌ | NOT-04 |
| **User profile** (role, company, focus areas) as AI context | ✅ | — | ❌ | NOT-05 |
| **"Edit by asking"** (rewrite notes or a selection through chat) | ✅ | ✅ (Oct 2026) | ❌ | NOT-06 |
| Rich-text editing of the summary | ✅ | ✅ | ✅ | — |
| **Circleback-style layout** (Action Items at top, Overview, topic sections, table of contents) | — | ✅ | ❌ | NOT-04 (template) and NOT-02 (TOC) |
| **AI meeting title** | ✅ | ✅ | ❌ | NOT-02 |
| **Suggested follow-up email drafts** (rules: external, 10 people or fewer; shown on Home for 3 days) | ✅ | ✅ | ❌ | NOT-07 |
| **Pre-meeting briefs** (cited 2–3 bullets for external meetings) | ✅ | ✅ (person/company summaries) | ❌ | NOT-08 |
| **Redaction of sensitive information** | ✅ | — | ❌ | NOT-09 |
| Images pasted into notes as context | ✅ | — | ❌ | NOT-01 |

### 5.4 Organization and entities

| Feature | G | C | Deino today | Card |
|---|---|---|---|---|
| **Calendar sync** (Google, Outlook; Apple is a Deino bonus) | ✅ | ✅ | ❌ | ORG-01 |
| **Home "Coming up"** (14 days), join button, email invitees, menu-bar next meeting and time left | ✅ | ✅ | ❌ | ORG-02 |
| **Auto-record rules per event and per series** (internal/external, organizer, RSVP) | 🟡 | ✅ | ❌ | ORG-02 |
| **Link a meeting to an event; related and recurring meetings; "review last meeting"** | ✅ | ✅ | ❌ | ORG-03 |
| **People and Companies pages** (meetings, open actions, upcoming events, AI summary, scoped chat) | ✅ | ✅ | ❌ | ORG-04 |
| **Folders** (nested, a note in many folders, auto-add for recurring series, folder chat) | ✅ | — | ❌ (one hard-coded list) | ORG-05 |
| **Tags** (colour, description, AI auto-tagging) | — | ✅ | ❌ | ORG-06 |
| **Meetings list:** filters (date, people, company, tag, folder), saved views, bulk actions | 🟡 | ✅ | ❌ | ORG-07 |
| **Trash, restore within 30 days** | ✅ | ✅ | ❌ (hard delete) | ORG-07 |
| **Merge meetings** | — | — | ❌ (requested in the OSS community) | ORG-07 |
| **Global search** (titles, notes, transcripts, people, @filters, moments), ⌘K navigation | ✅ | ✅ | 🟡 (SQL `LIKE` over transcripts) | ORG-08 |
| **Usage stats, activity heatmap, year in review** | ✅ (Crunched) | ✅ | ❌ | ORG-09 |

### 5.5 AI

| Feature | G | C | Deino today | Card |
|---|---|---|---|---|
| **Semantic index** | ✅ | ✅ | ❌ | AI-01 |
| **Chat across all meetings and any scope** (meeting, folder, person, company, selection) with inline citations and coverage | ✅ | ✅ | ❌ | AI-02 |
| **Chat during a meeting** ("what did I miss?") | ✅ | — | ❌ | AI-02 |
| **Chat history threads; model picker; streaming** | ✅ | ✅ | ❌ | AI-02 |
| **Web search and file attachments in chat** | ✅ | — | ❌ | AI-02 |
| **Chat write actions** (create or update action items, tags, notes, private notes on upcoming events) behind confirmation cards | ✅ | ✅ | ❌ | AI-03 |
| **Chat connectors to external MCP servers** (Linear, Notion, …) | — | ✅ | ❌ | AI-07 |
| **Recipes** (saved prompts, `/`, single vs multi-meeting, library, preview, share and remix) | ✅ | — | ❌ | AI-04 |
| **Structured action items** (assignee, **due date**, source link) | 🟡 (text only) | ✅ (no due date) | 🟡 (markdown table) | AI-05 |
| **Action Items page** (filters, views, bulk, mark all mine done, standalone items, reminders) | — | ✅ | ❌ | AI-05 |
| **AI insights** (custom typed fields, Auto/N count, speaker and timestamp attribution, global table, CSV) | — | ✅ | ❌ | AI-06 |
| **Rolling person and company AI summaries** | — | ✅ | ❌ | ORG-04 |
| **Weekly roundup** | — | ✅ | ❌ | AI-08 |
| **Email context** (Gmail/Outlook threads with attendees) and **draft/send email** | ✅ | ✅ | ❌ | INT-07 |

### 5.6 Automations, integrations, export, developer platform

The integrations here (INT-*) send data **directly from the Mac to that service** once the user connects it. There is no Deino server in between. Everything else in this table (automations engine, exports, MCP, local API, CLI) runs locally.

| Feature | G | C | Deino today | Card |
|---|---|---|---|---|
| **Automations engine** (condition groups with AND/OR, chained actions, auto or manual, re-run on past meetings, run log) | 🟡 (folder auto-post) | ✅ | ❌ | AUT-01 |
| **Export to Markdown, PDF, DOCX, TXT, JSON, SRT/VTT; rich copy** | 🟡 | 🟡 | ❌ (clipboard only) | EXP-01 |
| **Auto-export to a folder or Obsidian vault; full account export (zip, CSV)** | 🟡 (CSV) | ✅ | ❌ | EXP-02 |
| **OAuth and connections framework** | ✅ | ✅ | ❌ | INT-00 |
| **Slack:** post a note or action items (threaded) | ✅ | ✅ | ❌ | INT-01 |
| **Notion:** page or database with property mapping | ✅ | ✅ | ❌ | INT-02 |
| **Linear and task tools** (Asana, Jira, Todoist, monday, ClickUp, Trello, Apple Reminders, Things), with **two-way status sync** | via Zapier | ✅ (one-way) | ❌ | INT-03 |
| **CRMs:** HubSpot, Attio, Salesforce, Affinity, Zoho | ✅ | ✅ | ❌ | INT-04 |
| **Zapier / Make** (through webhooks) | ✅ | ✅ | ❌ | DEV-01 |
| **Google Docs, Apple Notes** | via Zapier | 🟡 | ❌ | INT-05 |
| **Gmail/Outlook email: context, drafts, send** | ✅ | ✅ | ❌ | INT-07 |
| **Outbound webhooks** (signed, retries, test send, field selection) | ✅ | ✅ | ❌ | DEV-01 |
| **Local MCP server** (read and write tools, start/stop recording) | ✅ | ✅ | ❌ | DEV-02 |
| **Local REST API** (keys, OpenAPI, import endpoint) | ✅ | ✅ | ❌ | DEV-03 |
| **CLI** (`deino …` with `--json`) | — | ✅ | ❌ | DEV-04 |
| **Deep links** (`deino://`), **global hotkeys**, Raycast, Shortcuts | — | ✅ | ❌ | DEV-05 |
| **Post-meeting shell hooks** (JSON on stdin) | — | — | ❌ (OSS peer Muesli has it) | AUT-01 |

### 5.7 Privacy, data and quality of life

| Feature | G | C | Deino today | Card |
|---|---|---|---|---|
| **Retention:** auto-delete audio, transcript or everything after N days | ✅ | ✅ | ❌ | PRV-01 |
| **Encryption at rest** (SQLCipher) and app lock | ✅ (SQLCipher) | ✅ | ❌ | PRV-02 |
| **Secrets in the Keychain** | ✅ | ✅ | ❌ (plaintext) | FND-06 |
| Telemetry and analytics | ⚠️ opt-out training | none | opt-in PostHog | FND-01 (remove) |
| **Dark mode** | ✅ | ✅ | ❌ | FND-10 |
| **Onboarding** (calendar, permissions including Accessibility, model download, sample meeting) | ✅ | 🟡 | 🟡 | FND-11 |
| Auto-update | ✅ | ✅ | ✅ (points at Zackriya) | FND-01 (disable or re-point) |

### 5.8 Won't build: needs a server, or the owner said no

| Feature | G | C | Why not | Local alternative |
|---|---|---|---|---|
| Meeting bot | — | ✅ | Owner decision: recording stays on the local computer | Desktop capture (CAP-*) |
| Share links / web viewer | ✅ | ✅ | Needs a hosted server | Export a PDF, DOCX or Markdown file and send it yourself (EXP-01) |
| Email recap sent automatically to attendees | — | ✅ | Needs a server | Follow-up email draft opened in Mail.app (NOT-07) |
| Multi-device sync | ✅ | ✅ | Needs a server | Auto-export to a folder you control (EXP-02) |
| iOS / Android / Watch apps | ✅ | ✅ | **Future consideration** (§10); build the Mac app first | Record in-person meetings on the Mac |
| Slack bot (ask in Slack, link unfurls) | ✅ | ✅ | Needs a server | Post notes to Slack yourself (INT-01, online opt-in) |
| Teams, workspaces, admin, SSO/SCIM, audit log | ✅ | ✅ | Single-user app | — |

---

## 6. Task cards

Card format:
- **Wave** (§7) · **Depends on** · **Size** (S ≈ ½ day of agent work, M ≈ 1–2 days, L ≈ 3–5 days, XL = split it further if you can)
- **Parity**: where the target behaviour is described in the research files (G = `granola.md`, C = `circleback.md`, CM = `community.md`)
- **Owns**: files, modules and tables this card may create or change substantially
- **Build**: what to build
- **Accept**: the acceptance checklist

### 6.0 Phase 0: Foundations (do these first; mostly one at a time)

#### FND-01 · Rebrand to Deino and remove upstream ties
- **Wave** 0 · **Depends on** — · **Size** M
- **Owns:** every file listed in `meetily_audit.md` §4, plus `README.md`, `CLAUDE.md` and `PRIVACY_POLICY.md`
- **Build:**
  - **Identity:**
    - Product name `Deino`, bundle id `com.xenshreyas.deino` (§8 D5).
    - Cargo package `deino`, npm package `deino`.
    - Window title, tray tooltip, notification titles, onboarding copy.
  - **Renamed paths and names:**
    - DB file → `deino.sqlite`.
    - Recordings folder → `~/Movies/Deino`.
    - Data dirs → app identifier dir only (drop `dirs::data_dir()/Meetily`).
    - `MEETILY_LLAMA_HELPER` → `DEINO_LLAMA_HELPER`.
    - Aggregate device → `deino-audio-tap`.
    - User agent → `Deino/<ver>`.
    - IndexedDB and localStorage keys → `deino*`.
    - Console log process name.
  - **Remove** PostHog completely: `analytics/` module, `lib/analytics.ts`, the consent UI, the `posthog-rs` dependency and all `track_*` commands.
  - **Updater:** disable it for now (§8 D6). Remove the updater endpoint and the Zackriya minisign key, but keep the plugin wired behind a setting.
  - **Downloads:**
    - Parakeet v3: download from HuggingFace `istupakov/parakeet-tdt-0.6b-v3-onnx` at a pinned revision, not from the Zackriya mirror. This is temporary; CAP-01 replaces the default engine.
    - FFmpeg at build time: download from an upstream source with a **sha256 pin**, or vendor it in this repo's releases.
  - **Remove** the About upsell, the Zackriya links, the PRO references and the LAUNCH20 coupon.
  - **New placeholder logo and icons:** a simple "D" mark in all sizes that `tauri icon` needs.
  - **README:** rewrite it as a short internal README.
  - **CLAUDE.md:** update it for Deino. Point at this plan, and drop the BlackHole claim, which is outdated.
- **Accept:**
  - [ ] `rg -i 'meetily|zackriya|meeting-minutes|meeting_minutes'` returns nothing outside `THIRD_PARTY_NOTICES.md`, `docs/research/`, migration history and this file.
  - [ ] The app builds and runs on a clean data dir, and onboarding completes.
  - [ ] No network calls to posthog, zackriya or towardsgeneralintelligence hosts.

#### FND-02 · Remove dead code, go English-only, register commands modularly
- **Wave** 0 · **Depends on** FND-01 · **Size** M
- **Owns:** `lib.rs`, `backend/` (delete), the dead files listed in `meetily_audit.md` §1.3, the language UI, and `summary/language_detection.rs`
- **Build:**
  - **Delete:**
    - `backend/` and `.gitmodules`.
    - `audio_v2/`, `lib_old_complex.rs`, `audio/stt.rs`, `core-old.rs`, `recording_saver_old.rs`, `*.backup`, `_stderr_suppressor.rs`, `system_audio_types.ts`.
    - `frontend/vs_buildtools.exe`, the committed logs, and `/notes/[id]`.
    - The legacy server commands (`api_get_profile` etc.) and the vestigial `serverAddress`.
    - The legacy and Homebrew DB import UI, plus the `licensing` table (new migration dropping it).
    - Cloud STT stubs, the unused `@remirror`/`@tiptap` dependencies, and the `electron` main field.
    - Clean the CSP of ports 5167 and 8178.
  - **Fix the broken invokes** `api_get_auto_generate_setting` and `builtin_ai_get_models_directory`: implement them or remove the callers.
  - **English-only:**
    - Remove the language picker, `set_language_preference`, summary translation and language detection (`whatlang`).
    - Hard-code transcription language `en`.
    - Keep the prompt "English normalization" only where it is harmless.
  - **Command registration (decided: inline plugins, §4.0):**
    - Move capabilities from `tauri.conf.json` into `capabilities/default.json`.
    - Move all legacy commands into `src/legacy/`, still under one `generate_handler!`, with a short `src/legacy.rs` doc saying they will be ported card by card. Don't add an `AppManifest`.
    - Create `features.rs`, `features/manifest.rs` and the `build.rs` `InlinedPlugin` loop (`architecture.md` §1 has the code).
    - Add a capability guard test asserting every manifest feature is granted.
    - FND-13 adds the first real plugin.
- **Accept:**
  - [ ] `cargo build`, `cargo clippy -D warnings` (new code only), and `pnpm run build` all pass.
  - [ ] Recording, import, summary and settings still work end to end.
  - [ ] `CLAUDE.md` documents the command-registration pattern, and ADR 0002 records it.

#### FND-03 · CI and quality gates
- **Wave** 0 · **Depends on** FND-13 · **Size** S
- **Owns:** `.github/workflows/`, `frontend/package.json` scripts
- **Build:**
  - Follow `architecture.md` (b) FND-03.
  - **One `ci.yml`** with path filters and `cancel-in-progress`.
    - macOS job: fmt, `scripts/clippy.sh`, `cargo nextest run --workspace`, the file-size check and `cargo tree` dependency-rule checks.
    - Linux job: `cargo check --workspace`.
    - Frontend job: `pnpm install --frozen-lockfile`, typecheck, lint, `vitest run`, `vite build`.
    - Bindings job: regenerate, then `git diff --exit-code`.
    - PR-title Conventional Commits check.
  - Use `Swatinem/rust-cache`. Add `lefthook.yml` for local hooks.
  - Delete or disable the upstream release workflows that reference S3/Zackriya secrets, and keep one manual `release-macos.yml` (unsigned DMG artifact).
- **Accept:**
  - [ ] A PR that breaks a test fails CI.
  - [ ] CI passes on `main`.

#### FND-04 · Test fixtures and mocked-IPC dev mode
- **Wave** 0 · **Depends on** FND-13 · **Size** M
- **Owns:** `frontend/src/dev/mock-ipc/`, `crates/deino-testkit/`, `frontend/src-tauri/src/dev_fixtures.rs`, `fixtures/`
- **Build:**
  - **Fixtures:** `fixtures/meetings/*.json` with 8 realistic English meetings:
    - 1:1, standup, sales discovery (external), interview, board update, customer support, in-person brainstorm, a 90-minute planning meeting
    - each with segments (channel, speaker, ms timestamps), attendees with emails and domains, calendar event data, and a short audio clip for 2 of them
  - **Rust:** a test helper `test_db()` that creates an in-memory SQLite with all migrations applied, plus `seed_fixtures(&pool)`.
  - **Debug-only command** `dev_seed_fixtures`, available from Settings → Developer when `debug_assertions` is on.
  - **`pnpm dev:mock` = `vite --mode mock`.** A `resolve.alias` swaps `@/lib/ipc/generated/*` for `@/dev/mock-ipc/*`. Each mock module `satisfies typeof generated.<feature>.commands`, so a stale mock fails typecheck. Mocks serve fixture data from an in-memory store. Each feature card adds its own mock module.
  - **`MockAudioSource`** (`deino-audio` test-support) plays fixture mic/system tracks. In debug builds it is selected with `DEINO_MOCK_AUDIO` (pattern: anarlog `crates/audio-mock`).
  - See `architecture.md` (b) FND-04.
- **Accept:**
  - [ ] `pnpm dev:mock` shows the meeting list and a meeting page with fixture data in a browser.
  - [ ] `test_db()` is used by at least one repository test.

#### FND-05 · Rust-owned live persistence and multitrack audio files
- **Wave** 1 · **Depends on** FND-02 · **Size** L
- **Owns:** `audio/recording_manager.rs`, `audio/recording_saver.rs`, `audio/incremental_saver.rs`, `hooks/useRecordingStart.ts`, `hooks/useRecordingStop.ts`, `services/storageService.ts`, `contexts/TranscriptContext.tsx`
- **Parity:** Circleback's crash-safe recording (C §3.5, reliability engineering)
- **Build:**
  - **Rust creates the meeting row** (`status='recording'`) at start and returns `meeting_id` to the UI.
  - Each finalized transcript chunk is written to `transcript_segments` **as it arrives** (the FND-07 table; coordinate by landing FND-07 first or in the same PR).
  - On stop, Rust sets `status='processing'` and enqueues the pipeline (FND-08). The frontend no longer saves transcripts.
  - **Audio files (decided by spike, `docs/references/spike-native-recording.md`):** write `mic` and `system` as separate tracks with **AVAssetWriter in CMAF segment-output mode**.
    - Format: AAC-LC, 64 kbps, 48 kHz, mono.
    - Segments are 1 s; a delegate appends each segment's bytes to Deino's own `audio/<track>.NNN.m4a.part`.
    - F_FULLFSYNC every 5 s, plus an atomic `recording.json` manifest holding the durable frame counts.
    - Recovery uses the box scanner `truncate_to_playable`.
    - At finalize, passthrough-export to flat `mic.m4a` and `system.m4a`, and build the mixed `audio.mp4` from the two tracks.
    - **No FFmpeg** (D11). Port the spike code from `~/Documents/GitHub/deino-spikes/native-recording/src/{encoder/cmaf.rs,encoder/asset_writer.rs,recover.rs}`.
    - **Pitfall:** symphonia 0.5.5 ignores AAC priming (44 ms). Decode Deino's own files with AVFoundation/ExtAudioFile.
  - **Recovery:** on startup, any meeting stuck in `recording` is recovered from checkpoints and moved to `processing`. Remove IndexedDB transcript recovery once this works.
- **Accept:**
  - [ ] Killing the app mid-meeting with `kill -9` loses at most about 1 s of audio (the spike measured 0.03–0.84 s) and about 5 s of transcript, and the meeting appears after relaunch.
  - [ ] The three audio files exist and are time-aligned within 50 ms.
  - [ ] No transcript writes come from the frontend.

#### FND-06 · Typed settings store and Keychain secrets
- **Wave** 1 · **Depends on** FND-02 · **Size** M
- **Owns:** `settings/` (new module), `api/api.rs` (settings parts), `ModelSettingsModal.tsx`
- **Build:**
  - **Settings store:**
    - A single typed `Settings` struct (serde, versioned) persisted in `settings.json` under the app config dir.
    - Commands `settings_get` and `settings_patch(json_patch)`, and an event `settings-changed`.
    - Frontend: a `useSettings()` hook.
    - Migrate the existing SQLite `settings`/`transcript_settings` rows and the tauri-store prefs into it, and remove the old tables (new migration).
  - **Secrets:**
    - A `secrets` module using the `keyring` crate (service `com.xenshreyas.deino`).
    - Commands `secret_set(name)`, `secret_has(name)` and `secret_delete(name)`. Values can never be read back by the UI.
    - Move every API key there.
- **Accept:**
  - [ ] There are no API keys in SQLite or JSON.
  - [ ] Existing users' keys are migrated on first launch.
  - [ ] Unit tests cover the settings migration.

#### FND-07 · Core schema
- **Wave** 1 · **Depends on** FND-02 · **Size** M
- **Owns:** the migration that creates every table marked FND-07 in §4.1, `database/models.rs`, `database/repositories/{meeting,segment,people,company,calendar,folder,tag,action_item}.rs`
- **Build:**
  - Create the tables in §4.1 with indexes on FKs, `meeting_id`, `start_ms`, `email` and `domain`.
  - Migrate existing `transcripts` rows into `transcript_segments` with `channel='mixed'`.
  - Write repositories with CRUD and list/filter queries. Keep the legacy `api_*` meeting commands working on top of the new tables.
- **Accept:**
  - [ ] Migration round-trip test passes on a DB seeded with old-format data.
  - [ ] Every repository has tests.

#### FND-08 · Post-meeting pipeline (job runner)
- **Wave** 2 · **Depends on** FND-05, FND-07 · **Size** M
- **Owns:** `pipeline/` (new), the `jobs` and `job_runs` tables
- **Build:**
  - The runner described in §4.2: concurrency of 1 per meeting and 2 globally, retry with backoff (3 attempts), cancellation, and an event `pipeline-progress {meeting_id, stage, status}`.
  - Commands: `pipeline_status(meeting_id)`, `pipeline_rerun(meeting_id, stage_id?)`.
  - Port the existing summary generation in as a temporary stage, `legacy_summary` (order 50), until NOT-02 replaces it.
  - UI: a small processing-status strip on the meeting page, showing each stage with a retry button on failure.
- **Accept:**
  - [ ] Stages run in order.
  - [ ] A failed stage can be re-run alone.
  - [ ] An app restart resumes pending jobs.
  - [ ] Tests use fake stages.

#### FND-09 · LLM gateway
- **Wave** 1 · **Depends on** FND-06 · **Size** L
- **Owns:** `llm/` (new; absorbs `summary/llm_client.rs`, `ollama/`, `openai/`, `anthropic/`, `groq/`, `openrouter/`), `llama-helper/src/main.rs`
- **Build:**
  - Everything in §4.3.
  - **Tool calling:** the Anthropic Messages tool-use format and OpenAI `tools`.
  - **Streaming:** SSE parsing, emitted to the UI as `llm-stream {request_id, delta}`.
  - **`structured<T>()`:** uses JSON-schema output (Anthropic tool trick, or OpenAI `response_format: json_schema`), with a repair retry.
  - **llama-helper** gains:
    - `Stream` (token events over stdout JSON lines)
    - `Grammar` (GBNF from a JSON schema)
    - `Embed` (embedding GGUF)
  - **Settings → AI page:** provider credentials, the per-route model table, and a "Test" button per route.
  - Port the summary code to the gateway.
  - Write the `llm_calls` usage table.
  - Provide a `MockProvider`.
- **Accept:**
  - [ ] Existing summaries work through the gateway with every provider.
  - [ ] Tool-calling loop tests run against `MockProvider`.
  - [ ] Streaming works against Claude and the built-in model.

#### FND-10a · Migrate the frontend to Vite + TanStack Router + Tailwind v4 + Vitest
- **Wave** 0 · **Depends on** FND-02 · **Size** M
- **Owns:** `frontend/package.json`, `frontend/vite.config.ts`, `frontend/vitest.config.ts`, `frontend/src/main.tsx`, `frontend/src/router.tsx`, `frontend/src/routes/`; deletes `next.config.js` and `src/app/`
- **Read:** `docs/references/architecture.md` §7 and (b) FND-10. Reference: `~/Documents/GitHub/deino-references/anarlog/apps/desktop/{vite.config.ts,src/main.tsx,src/routes/}`.
- **Build:**
  - **Vite + React plugin** with TanStack Router (file-based, `autoCodeSplitting`). Update the Tauri `devUrl` and `frontendDist`.
  - **Port the existing routes:**
    - `/` → `routes/index.tsx`
    - `/meeting-details?id=` → `routes/meetings/$meetingId.tsx`, keeping a redirect for the old URL
    - `/settings`
    - Delete `/notes/[id]`.
  - Replace `next/link`, `next/navigation`, `next/image` and the `metadata` exports.
  - **Remove the ProseMirror webpack alias hack.** BlockNote stays frozen until NOT-01 swaps it for TipTap.
  - **Tailwind v4** via `@tailwindcss/vite`. Keep the existing look for now; FND-10 restyles it.
  - **Tests:** move the 10 existing bun tests to Vitest (happy-dom, Testing Library) and add the anarlog `test-setup.ts` Tauri shim.
  - Add scripts: `dev`, `build`, `test`, `typecheck`, `lint`.
  - Remove `next`, `eslint-config-next` and the other Next-only dependencies.
- **Accept:**
  - [ ] `pnpm tauri dev` runs the app with every existing screen working: record, meeting details, settings, onboarding, import.
  - [ ] `pnpm build` and `pnpm test` pass.
  - [ ] No `next` packages remain.

#### FND-10 · App shell, navigation, design system, dark mode
- **Wave** 1 · **Depends on** FND-10a, FND-13 · **Size** L
- **Owns:** `frontend/src/app/layout.tsx`, `components/Sidebar/*` (replace), `frontend/src/features/shell/`, `globals.css`, `tailwind.config`
- **Build:**
  - Build the shell from §4.6 with a nav registry (`features/shell/nav.ts`, an append-only array of `{id, label, icon, href, order}`).
  - **⌘K palette framework** with a registry of commands and providers (ORG-08 adds search results to it).
  - Light, dark and system themes using CSS variables.
  - An empty-state component; a page-header component.
  - **Meeting page skeleton** with tab slots (Notes, Transcript, Insights, Clips) and right-panel slots (Chat, Participants, Details) that feature cards fill in through a registry (`features/meeting/slots.ts`).
  - Move the existing transcript and summary panels into the Transcript and Notes slots.
  - **Visual direction:** calm, Granola-like, lots of whitespace, one accent colour, Inter or SF system font.
- **Accept:**
  - [ ] All existing flows are reachable.
  - [ ] Dark mode has no unreadable text.
  - [ ] A new nav item or tab can be added by editing one registry file.
  - [ ] Screenshots of `dev:mock` are in the PR.

#### FND-13 · Code-quality baseline and workspace skeleton
- **Wave** 0 · **Depends on** FND-10a · **Size** L
- **Read:** `docs/references/architecture.md` §1–3, §8, §10, §11, and (b) FND-13 and (c), which has the exact config files to copy.
- **Owns:** `docs/ARCHITECTURE.md`, `docs/adr/`, `.github/pull_request_template.md`, `clippy.toml`, ESLint/Prettier config, `frontend/src-tauri/src/error.rs`, the IPC type generation, the TanStack Query setup
- **Build:**
  - **Workspace skeleton (§4.0):**
    - Create `crates/deino-domain`, `crates/deino-db` (move the existing migrations here) and `crates/deino-testkit`, each with a README (Role / Owns / Does Not Own / Invariants).
    - Create the other crates as empty stubs only when their owning card starts.
    - Set up `[workspace.lints]`, `[workspace.dependencies]`, the dev profile, `.cargo/config.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `deny.toml`, `typos.toml` and the file-size script.
  - **First real inline plugin:** port the meetings list commands into `crates/deino-core/src/meetings` (service, repo, model, error) and `frontend/src-tauri/src/features/meetings.rs`, with tauri-specta bindings and the TanStack Query hook. This is the worked example everyone copies.
  - **`docs/ARCHITECTURE.md`:** an accurate map of the system as it is after FND-02: processes, modules, data flow, schema and extension points.
  - **ADRs:** `docs/adr/0001-record-architecture-decisions.md` plus a template. Write ADRs for the decisions already taken in this plan: Tauri + Rust core, SQLite + sqlx, the pipeline-stage pattern, the LLM gateway with routes, and cloud LLM for notes with local transcription.
  - **`AppError`:** the shared `{code, message}` type, serializable to TS, with a conversion pattern from module `thiserror` enums.
  - **Generated IPC types:** set up `tauri-specta` (or `ts-rs`) so command types and DTOs generate into `frontend/src/lib/ipc/generated.ts`. CI fails if the generated file is stale. Convert one existing feature (meetings list) as the worked example.
  - **TanStack Query:** set up the provider plus a helper that invalidates queries on Rust events. Convert the meetings list as the example.
  - **Lint and format configs:** a `clippy.toml` with curated pedantic lints, ESLint (typescript-eslint strict, react-hooks), and Prettier. Fix or explicitly allow the existing violations so CI starts green.
  - **PR template** and Conventional Commits check.
  - **Coverage reporting** in CI: `cargo llvm-cov` and Vitest coverage, informational.
  - **Reference module:** `frontend/src-tauri/src/_example_feature/`, a tiny commands → service → repo + adapter-trait module with full docs and tests. Agents copy it as their starting point. Exclude it from release builds.
- **Accept:**
  - [ ] The example module and the converted meetings list follow every §3A rule.
  - [ ] CI enforces lint, format, generated types and tests.
  - [ ] `ARCHITECTURE.md` is reviewed against the code.

#### FND-12 · Network client and data-egress controls
- **Wave** 1 · **Depends on** FND-06 · **Size** S
- **Owns:** `net/` (new), the Settings → Privacy → Network page
- **Build:**
  - **One HTTP client:** a shared `net::Client` that every Rust module must use for outbound requests. Add a CI check that fails on direct `reqwest::Client` use elsewhere.
  - **Service registry:** every outbound host belongs to a named service (`llm.anthropic`, `llm.openai`, `slack`, `notion`, `gmail`, `google_calendar`, `hf_models`, …). A request to an unregistered host fails.
  - **Per-service gating:** a service sends nothing until the user connects or enables it.
  - **Hard rule:** the transcription and audio modules get **no** network client injected, so they cannot make requests at all.
  - **Network log:** Settings → Network lists every outbound request (time, service, host, bytes, with no bodies) so the user can see exactly what left the Mac.
  - **Optional "Offline mode"** switch (OFF by default) for times the user wants zero traffic. Notes then fall back to the local model.
  - **Frontend CSP:** `connect-src` restricted to `self`, `ipc:` and `asset:`. All online calls go through Rust.
- **Accept:**
  - [ ] With no services connected, a full meeting (record → notes with the local model → export) makes zero outbound connections. Verify with `nettop` or Little Snitch and record the result in the PR.
  - [ ] Transcription has no code path to the network (compile-time: no client in scope).
  - [ ] The CI check catches direct `reqwest` use.

#### FND-11 · Onboarding v2
- **Wave** 4 · **Depends on** CAP-01, ORG-01, CAP-06 · **Size** M
- **Owns:** `components/onboarding/*`, `onboarding.rs`
- **Build:**
  - **Steps:**
    1. Welcome.
    2. Permissions: mic, system audio, notifications, **Accessibility** (optional, for speaker names and the consent message), **Calendar** (EventKit).
    3. AI setup: download Parakeet Unified EN and the embedding model, then paste a Claude or OpenAI API key for notes and chat (or download the built-in local model instead).
    4. Connect calendar.
    5. Profile (NOT-05).
    6. Try it: record a 20 s test or open the sample meeting from fixtures.
  - Every step can be skipped and resumed later from Settings.
- **Accept:**
  - [ ] A fresh install reaches a working first recording in under 3 minutes on a good connection.

### 6.1 Phase 1: Capture

#### CAP-01 · Parakeet Unified EN 0.6B engine (default)
- **Wave** 1 · **Depends on** FND-02 · **Size** M
- **Owns:** `audio/transcription/parakeet_unified_provider.rs` (new), `audio/transcription/engine.rs`, the model manager UI
- **Build:**
  - Port the integration from Handy (`~/Documents/GitHub/deino-references/Handy/src-tauri/`). Read `docs/REFERENCES.md` → "Handy STT" first.
    - Dependency: `transcribe-cpp = { version = "0.3.1", default-features = false, features = ["serde"] }`, plus `metal` on macOS.
    - Adopt Handy's **worker-process isolation** (`src/engine_supervisor/`), so a GGML abort or hang can't take down the app.
    - Adopt the FIFO stream feed/finalize lifecycle with a **batch fallback** on the retained audio.
    - Adopt the idle unload, the `GGML_METAL_NO_RESIDENCY=1` setting, the background Metal shader pre-warm, and the resumable sha256-verified downloader.
    - **Meeting-specific differences from Handy:**
      - Feed the stream **wall-clock-continuous audio**: do not VAD-gate the input, so timestamps stay on meeting time. VAD is used only for segmentation and UI.
      - Read **token timestamps** from the crate API (Handy doesn't). Check the crate source after `cargo fetch`.
      - **Two concurrent streams** for the mic and system lanes (CAP-02). A loaded model accepts only one stream (a second gets `Error::Busy`), so run **two worker processes**, about 1.5 GB at Q8_0. On Macs with 16 GB or less RAM, default to Q5_K_M, or fall back to alternating the lanes.
      - **Drop `whisper-rs`.** It and `transcribe-cpp` each bundle their own ggml, which risks a link clash. Run Whisper (when chosen) through `transcribe-cpp`, as Handy does.
      - **Latency:** the default stream window gives about 2.1 s to first text. Tune the commit policy and window to reach about 1.5 s without losing accuracy, and record what you measured.
      - Verify sha256 on the HuggingFace download path too.
  - **Model:** `handy-computer/parakeet-unified-en-0.6b-gguf` at revision `7e948f21b7bdbac698d3318db9d350f1096f3b6c`. Default quant `Q8_0` (731,357,568 bytes, sha256 `4b50b6dd…f38795`; copy all six quant entries and hashes from Handy's catalog). Download from HuggingFace with resume and sha256 verification.
  - **Streaming mode for live transcription**, with token timestamps stored in `words_json`.
  - Implement `TranscriptionProvider`.
  - Make it the default for new installs and keep Whisper and Parakeet TDT as alternatives under "Advanced".
  - Add Handy's notice to `THIRD_PARTY_NOTICES.md`.
- **Accept:**
  - **Spike-decided parameters** (`docs/references/spike-stt-streaming.md`):
    - streaming window (5600, 560, 320) ms, giving about 0.85 s onset-to-text at the same WER;
    - batch only on VAD chunks of 30 s or less (batch `run()` can stop partway through long files);
    - roll the stream every 2–5 min at a silence;
    - group tokens into words yourself;
    - Q8_0 at 32 GB RAM or more, otherwise Q5_K_M;
    - load timeout of at least 60 s (Metal shader compile);
    - **whisper-rs must be removed** (it aborts at runtime alongside transcribe-cpp).
  - **Model licence: NVIDIA Open Model License** (not CC-BY). Commercial use and redistribution are allowed. Ship a `NOTICE` entry with the exact text "Licensed by NVIDIA Corporation under the NVIDIA Open Model License", link the agreement (https://www.nvidia.com/en-us/agreements/enterprise-software/nvidia-open-model-license/) in `THIRD_PARTY_NOTICES.md` and the About screen, and don't bypass model guardrails.
  - [ ] Live transcript appears within about 2 s of speech on an M-series Mac (target 1.5 s), measured and logged with Handy-style `StreamPerf` metrics.
  - [ ] Import and retranscribe use the same engine.
  - [ ] Real-time factor is logged.

#### CAP-02 · Me/Them channel separation
- **Wave** 2 · **Depends on** FND-05, CAP-01 · **Size** L
- **Owns:** `audio/pipeline.rs`, `audio/transcription/worker.rs`, the segment `channel` writes
- **Parity:** G §2.1, G §4 (Me/Them); CM §4.4 (#241, #642; upstream PR #823 multitrack)
- **Build:**
  - Run VAD and STT **per channel**: two worker lanes, mic and system, and tag every segment with its `channel`. The mixed stream is still saved for playback.
  - **Echo and bleed suppression:** when the user is on speakers, the mic picks up system audio.
    - **Primary:** acoustic echo cancellation. Run a WebRTC AEC (e.g. the `webrtc-audio-processing` crate; check its licence) in 10 ms chunks, with the **system track as the reference stream** and the mic as the capture stream (pattern: Zed `livekit_client/playback.rs:50-80`, reference only).
    - **Fallback:** if AEC leaves residue, **hold back** mic finals while the system channel is speaking (3 s) and drop them **only on a text match** with system-channel text (normalized similarity > 0.6) — the OpenWhispr holdback pattern.
    - **Never drop mic audio on correlation alone:** OpenWhispr's field data shows real double-talk correlates 0.73–0.81 with the system audio.
    - **Run VAD on the AEC-cleaned mic** (the Muesli pattern).
  - **Timeline:** use the sample-count timeline with gap filling, and anchor the system track to the recording epoch so both tracks align at t=0. See `docs/REFERENCES.md` → "Cap/Zed audio patterns".
  - Add a setting "I use headphones" that skips suppression.
  - Default speakers: mic → the "Me" speaker (`is_me=1`, linked to the `people.is_me` row); system → "Them" until CAP-03 runs.
  - Update the live UI to label lines Me/Them (full layout in TRN-01).
- **Accept:**
  - [ ] On a Zoom call with headphones, every line is attributed correctly.
  - [ ] On laptop speakers, fewer than 5% duplicated lines on a 10-minute fixture recording (add the fixture).
  - [ ] CPU stays below 1.6× the single-lane baseline.

#### CAP-03 · Local speaker diarization
- **Wave** 3 · **Depends on** CAP-02, FND-08 · **Size** L
- **Owns:** `diarization/` (new), pipeline stage `diarize`, the `speakers` writes
- **Parity:** C §6 (Participant N, hybrid-room separation); G §2.6 (Speaker A/B)
- **Build:**
  - **Decided by spike (`docs/references/spike-diarization.md`):** anarlog engine + **WeSpeaker ResNet34-LM** embeddings (CC-BY-4.0, attribute it) loaded via the **`ort` CoreML execution provider**, segmentation on CPU, and **always pass calendar speaker bounds**.
    - Step 2 s, or 3 s for meetings over 30 min. Threshold 0.7045 (anarlog's AMI-tuned value), `min_cluster_seconds` 3 for clips under 2 min, else 12.
    - Batch the per-window embeddings.
    - Measured: 8–12 % DER with known counts; RTF 0.023 on 57 min.
    - Vendor `knf-rs`. Pin the model sha256s from the report.
  - **Primary choice: port Anarlog's `crates/pyannote-local` plus `crates/embedding` plus `crates/voiceprint` (MIT).**
    - It uses the same `ort` that Deino already depends on, returns the per-speaker centroid embeddings CAP-04 needs, and ships a DER eval binary.
    - Attribute the embedding model (probably WeSpeaker, CC-BY-4.0) in `THIRD_PARTY_NOTICES.md`.
    - Its own eval allows RTF 0.15, but the 60-minutes-in-2-minutes target needs about 0.033, so profile and tune early (batching, threads, `window_shift`).
  - **Alternative:** the official `sherpa-onnx` crate 1.13.8 (Apache-2.0, static link, CPU provider), not `sherpa-rs`. Models: pyannote `segmentation-3.0` plus ONE frozen English speaker-embedding model, chosen by measured DER on fixtures (candidates: CAM++ VoxCeleb, WeSpeaker ResNet34-LM, TitaNet-small). Download models on demand with sha256 pins. Read `docs/REFERENCES.md` → "Diarization" first.
  - **Scaling:** use `window_shift_ratio` 0.25 or more for meetings over 30 minutes; the clustering memory grows with the square of the number of embeddings.
  - **Progress bar:** add an `extern "C"` binding for `SherpaOnnxOfflineSpeakerDiarizationProcessWithCallback`.
  - **Linking:** check that `ort` and sherpa's static onnxruntime link together.
  - **Post-meeting stage:**
    - Diarize the **system** track into Speaker 1..N.
    - Also diarize the **mic** track when "In-person meeting" is on (a toggle at record start, auto-on when the system track is silent for over 80% of the meeting).
    - Assign each segment to a speaker by maximum time overlap. Split segments at speaker changes using word timestamps.
  - **Live (best effort):** an online clustering over embeddings of finalized system segments, so speakers show during the meeting. The post-meeting pass overrides it.
  - Store a speaker centroid embedding on `speakers` for CAP-04.
- **Accept:**
  - [ ] Diarization error rate (DER) is at most 20% on two 3–4 speaker fixtures (document how it was measured).
  - [ ] A 60-minute meeting diarizes in under 2 minutes on an M1.

#### CAP-04 · Speaker naming and voice profiles
- **Wave** 4 · **Depends on** CAP-03, ORG-04 · **Size** M
- **Owns:** the `voiceprints` table, `diarization/identify.rs`, the participants panel UI
- **Parity:** C §6 (voice profiles, rename this segment or all segments), CM §4.4 (#226/#230)
- **Build:**
  - **Participants panel:** colour, name and speaking-time % for each speaker.
  - **Rename a speaker:**
    - Pick from attendees or people, or type a new name (creates a person).
    - Choose "this segment" or "all segments".
    - Renaming applies everywhere: transcript, notes (regenerate prompt), action-item assignees and insights.
  - **Merge two speakers.**
  - **On rename-all**, save a voiceprint (the speaker centroid) for that person, keeping up to 5 per person.
  - **Stage `identify_speakers`:**
    - Match against voiceprints (max over up to 5 exemplars per person). **Auto-assign at ≥ 0.72 (≥ 0.77 for non-attendees) + margin ≥ 0.08 + mutual best; suggest 0.55–0.72** (spike-measured, WeSpeaker r34). Thresholds live in typed settings and get calibrated on real meetings. Add phantom-cluster pruning and known-voiceprint merging.
    - **Prefer people who are calendar attendees.**
    - The mic speaker is always Me.
  - Settings → People → Voice profiles: list and delete.
- **Accept:**
  - [ ] After naming someone once, a later fixture meeting with that person is auto-labelled.
  - [ ] Renames propagate to notes after regeneration.

#### CAP-05 · Speaker names from the meeting app UI (Accessibility)
- **Wave** 5 · **Depends on** CAP-03, CAP-06 · **Size** L
- **Owns:** `speaker_tags/` (new, macOS only)
- **Parity:** G §4 (speaker tags for Zoom desktop, Teams desktop, Meet in Chrome)
- **Build:**
  - **Watcher:** while recording, if Zoom, Teams or Chrome (with a Meet tab) is running and Accessibility permission is granted, poll the app's AX tree every 250 ms for:
    - the participant display names
    - the **active-speaker indicator**
  - Write a timeline of `(t_ms, name)` and align it with system-channel segments. A name wins a segment if it was active for more than 50% of the segment.
  - Use these names as strong priors for CAP-04 (they override voice matching).
  - Write a per-app adapter with fixture AX dumps so it can be unit tested.
  - Settings → Speaker tags shows a row per app with its Enabled / Needs permission / Unsupported state.
- **Accept:**
  - [ ] On real Zoom and Meet calls (manual QA), at least 90% of segments get the right name for speakers who talk for more than 3 s.
  - [ ] Unit tests pass on recorded AX fixture dumps.

#### CAP-06 · Meeting detection and auto-start
- **Wave** 2 · **Depends on** FND-05 · **Size** M
- **Owns:** `audio/system_detector.rs` (rewrite), `detection/` (new), the detection prompt window
- **Parity:** G §2.3 (per-app titles, 15-minute association window), C §3.5 (ignore list, countdown auto-start, camera signal); CM §5 (Muesli camera+mic, Platypus process heuristics); upstream PR #733
- **Build:**
  - **macOS signals:**
    - Per-process mic use through Core Audio process objects: `kAudioHardwarePropertyProcessObjectList`, then `kAudioProcessPropertyIsRunningInput` and `kAudioProcessPropertyBundleID`.
    - Camera use through `kCMIODevicePropertyDeviceIsRunningSomewhere`.
  - **Classify the app** from a bundle-id table: Zoom, Teams, Meet (browser plus window title), Slack ("Huddle detected"), FaceTime and WhatsApp ("Call detected"), Webex, Discord, others ("Meeting detected").
  - **Prompt:** a small always-on-top window with the title, the app name, **Take notes**, **Ignore** and **Always ignore this app**.
    - If a calendar event started less than 15 minutes ago or starts within 5 minutes, show the event name and link to it.
  - **Settings:** detection on/off, ignore list, and "Auto-start after N-second countdown" (off by default) with a **Don't record** button during the countdown.
  - Suppress the prompt while already recording.
- **Accept:**
  - [ ] Joining a Zoom or Meet call shows the prompt within 3 s.
  - [ ] Ignored apps never prompt.
  - [ ] Auto-start works with the countdown.

#### CAP-07 · Auto-stop and meeting boundaries
- **Wave** 3 · **Depends on** CAP-06, ORG-01 · **Size** M
- **Owns:** `detection/autostop.rs`
- **Parity:** G §2.5, C §3.5
- **Build:**
  - **Stop when:**
    - the meeting app releases the mic for 20 s or more (show a 15 s "Ending…" toast with a **Keep recording** button);
    - there are 15 minutes with no speech;
    - the system sleeps;
    - or (optional) the calendar end time is passed by 10 minutes with no speech in the last 2 minutes.
  - **Back-to-back meetings:** if the next calendar event starts while recording and its attendees differ, prompt "Start new note for <next event>?", which splits into a new meeting at that moment.
  - **Resume after stop:** a "Resume" button on a finished meeting appends new audio and segments (the meeting moves back to `recording`).
- **Accept:**
  - [ ] Each stop path is covered by a unit test with a simulated clock.
  - [ ] Manual QA checks one real call.

#### CAP-08 · Recording-only mode and post-pass transcription
- **Wave** 3 · **Depends on** FND-08, CAP-01 · **Size** S
- **Owns:** pipeline stage `final_transcription`
- **Parity:** CM §4.4 (#338, #729, #677; upstream PR #532)
- **Build:**
  - Setting "Live transcription: On / Off (transcribe after meeting)".
  - When it is off, record audio only and the stage transcribes the mic and system tracks after the meeting.
  - Optional "High-accuracy re-pass after meeting" (default off), which re-transcribes and replaces the live segments while keeping speaker assignments where they overlap.
- **Accept:**
  - [ ] With live transcription off, the CPU stays below 15% while recording.
  - [ ] Segments appear after stop.

#### CAP-09 · Screen recording, shared-screen snapshots, screen context
- **Wave** 4 · **Depends on** FND-08, NOT-02 · **Size** L
- **Owns:** `screen/` (new, macOS), the `snapshots` table, pipeline stage `screen_context`
- **Parity:** G §2.10 (snapshots, consent card, per-meeting off switch), C §3.5 (window video, on by default for detected meetings), C §5 (screen-share content in notes)
- **Build:**
  - **ScreenCaptureKit:**
    - An optional window or screen video track (`video.mp4`, H.264 at 1 fps for screen shares or a user-chosen rate) and a picker in the record panel.
    - **Snapshots:** every 10 s, capture the meeting window, keeping only frames where the perceptual hash changed by more than a threshold. Save them as PNG.
  - **Stage `screen_context`:**
    - OCR each kept snapshot with Apple Vision (`VNRecognizeTextRequest`, via `objc2-vision`).
    - Optionally describe it with a multimodal LLM route `vision.describe` (Claude) when enabled.
    - Store the text on the snapshot rows. NOT-02 includes it as context (time-aligned).
  - **UI:** an "Images" strip on the meeting page; click to enlarge; delete.
  - **Consent reminder** the first 3 times.
- **Accept:**
  - [ ] A recorded slide deck's text shows up in the enhanced notes.
  - [ ] Turning it off for one meeting works.
  - [ ] Paused while the window is hidden.

#### CAP-10 · Record panel, recording health, menu bar
- **Wave** 2 · **Depends on** FND-05, FND-10 · **Size** M
- **Owns:** `tray.rs`, the `record_panel` window (new Tauri window), `features/recording/`
- **Parity:** G §2.4 (nub, green bars, orange warning, menu-bar time left), C §3.5 (always-visible minimal panel that expands on hover, notes window)
- **Build:**
  - **Floating panel:**
    - An always-on-top, draggable, edge-snapping Tauri window shown while recording.
    - Collapsed, it shows a pulsing level meter and the elapsed time.
    - On hover it expands to pause/resume, stop, a mic picker, a system-audio toggle and **Open notes**.
    - Clicking it returns to the main window.
  - **Health monitor:** warn (orange state plus a notification) if:
    - no audio frames for 5 s on either track,
    - a VAD-active mic with no transcript for 30 s (when live transcription is on),
    - the model has crashed, or
    - disk space is under 2 GB.
  - **Menu bar:** next meeting and countdown, "Xm left" while recording, today's meetings with Join / Record / Open (ORG-02 provides the data).
  - Switch mic and toggle system audio mid-recording (extend `recording_manager`).
- **Accept:**
  - [ ] Unplugging the mic or killing the STT worker shows the warning within 10 s.
  - [ ] The panel never steals focus.

#### CAP-11 · Import v2 and folder watcher
- **Wave** 3 · **Depends on** FND-08, ORG-03 · **Size** S
- **Owns:** `audio/import.rs`, `components/ImportAudio/*`, `watch/` (new)
- **Parity:** C §3.9; CM §5 (Scriberr watcher)
- **Build:**
  - Promote import out of beta.
  - **Decoding (spike-proven):** use symphonia with features `alac`, `aiff` and `caf`, plus `symphonia-adapter-libopus` 0.2 for Opus. On any probe or codec error, fall back to AVFoundation `AVAssetReader` (48 kHz mono f32). No FFmpeg. See `docs/references/spike-native-recording.md` §4.
  - Accept video files (mov, mp4, mkv, webm), keeping the video for playback.
  - Optionally link the import to a calendar event (ORG-03 picker).
  - Imports go through the full pipeline.
  - **Watched folders** (settings list): new audio or video files that have been stable for 10 s are imported automatically; a "Move to processed" option is available.
- **Accept:**
  - [ ] Dropping a 2-hour mp4 into a watched folder produces a finished meeting.

#### CAP-12 · Importers from other note-takers
- **Wave** 5 · **Depends on** DEV-03, ORG-04 · **Size** M
- **Owns:** `importers/` (new)
- **Parity:** C §3.10
- **Build:**
  - Importers that take a user-pasted API key and map notes, transcript (with speakers and timestamps), attendees and action items into Deino:
    - **Granola** (public API `GET /v1/notes?include=transcript`, G §9.2)
    - **Circleback** (API `GET /meetings`, `/meeting/{id}`, `/meeting/{id}/transcript`, C §12.1)
    - **Fireflies** (GraphQL API key)
    - **Fathom** (API key)
  - Also a generic **JSON/Markdown folder import**.
  - Settings → Import shows a preview count, then runs in the background with progress. De-duplicate by source id.
- **Accept:**
  - [ ] Each importer is tested against recorded API fixtures.
  - [ ] Imported meetings are searchable and chat-able.

#### CAP-13 · Consent and transparency tools
- **Wave** 6 · **Depends on** CAP-05 · **Size** M
- **Owns:** `consent/` (new)
- **Parity:** G §10.3
- **Build:**
  - **Automatic chat message:**
    - Once transcription starts and speech is detected, post a configurable message in the meeting chat: Zoom and Teams through AX (focus the chat composer only if it is empty, type, send), Meet through AX in Chrome.
    - Off by default; per-app toggles.
  - **Consent reminder card** when recording starts.
  - **Watermark camera** (stretch): a CoreMediaIO camera extension that overlays "Transcribing". It needs a system extension and signing, so mark it as a **stretch goal** and stop to ask the owner before starting it.
- **Accept:**
  - [ ] Manual QA posts the message in Zoom and Meet test calls.
  - [ ] The message never sends while the user is typing.

### 6.2 Phase 1: Transcript

#### TRN-01 · Transcript view v2
- **Wave** 3 · **Depends on** CAP-02, FND-10 · **Size** M
- **Owns:** `features/transcript/` (replaces `VirtualizedTranscriptView`/`TranscriptView`)
- **Parity:** G §4 (bubbles, per-chunk copy and delete, search), C §7.1 (word highlight, search by speaker, counter)
- **Build:**
  - Virtualized list grouped into speaker turns.
  - **Me** bubbles on the right (accent colour); everyone else on the left in their speaker colour.
  - Speaker name and timestamp on each turn.
  - **⌘F search:** highlight hits, Enter or Shift+Enter to move between them, an "n of N" counter, and a `speaker:` filter.
  - Hover actions: Copy, Delete (TRN-02), Play from here (TRN-03), Comment (TRN-06), Clip (TRN-04).
  - **Live mode:** auto-scroll, plus a "Jump to now" button when the user has scrolled up.
- **Accept:**
  - [ ] Smooth with 5,000 segments.
  - [ ] Search across a 2-hour meeting returns in under 100 ms.

#### TRN-02 · Transcript editing, deletion, search and replace
- **Wave** 4 · **Depends on** TRN-01 · **Size** M
- **Owns:** segment update commands, `features/transcript/edit/`
- **Parity:** G §4 (delete chunk, then a regenerate prompt), CM §4.4 (#377, #357, #753)
- **Build:**
  - Inline edit of segment text (double-click). The edit sets `edited_at`; keep the original in an `original_text` column.
  - Delete a segment, which soft-deletes it and excludes it from AI and search.
  - **Find and replace** across one meeting, with an option for all meetings (preview with counts, then apply).
  - After any edit, show a banner "Transcript changed: Regenerate notes?".
  - Re-index affected segments.
- **Accept:**
  - [ ] Edits persist.
  - [ ] Deleted text never appears in chat answers or search (test it).

#### TRN-03 · Synced audio and video player
- **Wave** 3 · **Depends on** FND-05, TRN-01 · **Size** M
- **Owns:** `features/player/` (fill or replace `AudioPlayer.tsx` and `useAudioPlayer.ts`)
- **Parity:** C §7.1, C §7.6
- **Build:**
  - Docked player: play/pause, ±15 s, speed 0.75–2×, scrubber with speaker-coloured activity bands, a track selector (Mixed/Me/Them), and video when present.
  - **Sync:** highlight the current segment (and word, if `words_json` exists); click a segment or word to seek; keyboard Space and ←/→.
  - Download the audio or video (save dialog).
  - Serve files through the Tauri asset protocol, scoped to the recordings dir. Remove the broad `fs:read-all` capability if nothing else needs it.
- **Accept:**
  - [ ] Seeking is accurate to ±200 ms on a fixture with known timestamps.

#### TRN-04 · Clips and moments
- **Wave** 4 · **Depends on** TRN-03 · **Size** S
- **Owns:** the `clips` table, `features/clips/`
- **Parity:** C §7.4, C §7.5
- **Build:**
  - **Moments:** select transcript text → "Copy link" gives `deino://meeting/<id>?t=<ms>` (DEV-05 handles the link).
  - **Clips:** select a range → name it → the clip is saved and exported to `clips/<name>.m4a` (or mp4) with `AVAssetExportSession` (Passthrough preset + `timeRange`; spike measured a sample-exact cut in 11 ms; code at `deino-spikes/native-recording/src/clip.rs`).
  - Clips tab on the meeting page, with play, rename, reveal in Finder and delete.
  - A global clips list in the ⌘K palette.
- **Accept:**
  - [ ] A clip's audio matches the selected range ±250 ms.

#### TRN-05 · Custom vocabulary and corrections
- **Wave** 3 · **Depends on** FND-08 · **Size** S
- **Owns:** the `vocab_terms` table, pipeline stage `vocab_fixups`
- **Parity:** G §4 (jargon of 30–50 terms), C §5 (vocabulary, names included automatically); CM §4.4 (#474, #824)
- **Build:**
  - Settings → Vocabulary: terms with optional "sounds like" variants and replacement rules.
  - Automatically include the names of attendees, people and companies.
  - If the engine supports hotword boosting, pass the terms through. If not (likely), apply the fixups stage: case-insensitive whole-word replacement of the variants, plus fuzzy matching (Jaro-Winkler 0.92 or higher) for multi-word names.
  - Apply to live segments as well (cheap pass).
- **Accept:**
  - [ ] A fixture with "Shreyas" misheard as "Sreyas" is corrected.
  - [ ] There are no replacements inside other words.

#### TRN-06 · Comments on transcript and notes
- **Wave** 5 · **Depends on** TRN-01 · **Size** S
- **Owns:** the `comments` table, `features/comments/`
- **Parity:** C §7.3
- **Build:**
  - Select text in the transcript or notes → comment, with threaded replies, a resolve action, and a highlight on the commented span.
  - Single-user use: comments are private annotations.
- **Accept:**
  - [ ] Comments survive transcript re-indexing (anchored by segment id and offset).

### 6.3 Phase 2: Notes

#### NOT-01 · Live notepad ("My notes")
- **Wave** 2 · **Depends on** FND-05, FND-10 · **Size** M
- **Owns:** `meeting_notes` usage, `features/notepad/`
- **Parity:** G §3.1 (editor, markdown shortcuts, checkboxes, images, @mentions, focus mode), C §7.2 (private notes before, during and after)
- **Build:**
  - A **TipTap 3** editor (replacing BlockNote; remove BlockNote and its dependencies here) shown **on the recording screen and the meeting page** (tab "My notes"), also reachable from the record panel's **Open notes** window.
  - **Editor features:** markdown shortcuts, headings, bullets, `[]` checkboxes, bold/italic, ⌘⇧V paste as plain text, image paste and drag (saved to the meeting folder), and **@mentions** of people (inserts a person chip).
  - Autosave every 1 s, debounced, to `meeting_notes.notes_json` and `notes_markdown`.
  - **Before a meeting:** opening an upcoming calendar event creates its meeting row (`status='scheduled'`) so notes and agenda can be written ahead of time (ORG-02 links it in).
  - **Focus mode:** hide the window chrome while typing.
  - Remove the old "Add context for AI summary" textarea, since the notepad replaces it.
- **Accept:**
  - [ ] Notes typed during recording survive a crash.
  - [ ] Images render after relaunch.

#### NOT-02 · Enhanced notes engine (Granola-style)
- **Wave** 3 · **Depends on** NOT-01, FND-08, FND-09, NOT-04, NOT-05 · **Size** XL (split into 02a engine, 02b renderer)
- **Owns:** `notes/enhance/` (new; replaces `summary/processor.rs` and `service.rs` usage), the `enhanced_notes` table, `features/notes/`, pipeline stages `enhance_notes` and `auto_title`
- **Parity:** G §3.2–§3.3, C §5 (structure, inputs)
- **Build (02a, engine):**
  - **Inputs:**
    - the transcript (with speaker names and channel),
    - the user's notes (**treat everything the user typed as important and keep their wording**),
    - calendar title, attendees and description,
    - the user profile (NOT-05),
    - the template (NOT-04),
    - screen context (CAP-09) and vocabulary,
    - recent previous meetings in the same series (the last notes only).
  - **Output:** a `structured<EnhancedDoc>` (converted to TipTap JSON by `deino-doc`) call where `EnhancedDoc = { title, sections:[{heading, blocks:[{kind: bullet|para|todo, text, origin: user|ai, evidence:[{segment_id, start_ms}]}]}] }`.
    - The prompt must quote user-origin lines verbatim, place them under the right section, and add AI detail beneath.
  - **Long meetings:** map-reduce over 20-minute windows (keep the existing chunking ideas) and keep evidence ids through the reduce step.
  - Validate that every evidence `segment_id` exists. Drop citations that don't.
  - **Auto title** (`title` route) when the meeting has no calendar title or the title is generic.
  - **Triggering:** automatically when the pipeline runs, plus an **Enhance** button.
  - **Regenerating** keeps old versions (version history dropdown). **Confirm before regenerating** when `is_user_edited`.
  - **Regenerate prompts** appear when attendees change, transcript chunks are deleted or edited, the user's notes are edited after enhancement, or new audio is appended.
- **Build (02b, renderer):**
  - TipTap custom mark for origin (`origin=user|ai`), plus a citation mark or node, converted to and from Markdown in Rust by `deino-doc`: **user text in the primary colour, AI text in a muted grey** (both readable in dark mode).
  - Fully editable. Editing an AI block flips it to user-edited (keep it grey but mark it edited).
  - Table of contents on the right for documents with three or more headings.
  - Copy as Markdown or rich text.
- **Accept:**
  - [ ] Every user-typed line appears verbatim in the output (test with the mock provider plus a real-provider eval on 3 fixtures).
  - [ ] Over 90% of AI bullets carry at least one valid citation.
  - [ ] A 90-minute meeting completes in under 2 minutes with Claude.

#### NOT-03 · Citations UI
- **Wave** 4 · **Depends on** NOT-02, TRN-03 · **Size** S
- **Owns:** `features/notes/citations/`
- **Parity:** G §3.2 (magnifying glass shows the transcript source)
- **Build:**
  - On hover, AI blocks show a 🔍 icon. Clicking it opens a popover with the cited transcript excerpt(s), the speaker and time, a **Play** button (seeks the player) and **Show in transcript**.
- **Accept:**
  - [ ] Works for multi-citation bullets.
  - [ ] Keyboard accessible.

#### NOT-04 · Templates v2
- **Wave** 2 · **Depends on** FND-07, FND-10 · **Size** M
- **Owns:** the `templates` table (moves `frontend/src-tauri/templates/*.json` into seeds), `features/templates/`
- **Parity:** G §3.4, CM §4.4 (#243, #422)
- **Build:**
  - **Template model:** `{id, name, description, meeting_context, sections:[{title, instructions, format: bullets|paragraph|table|todo}], is_builtin, auto_rules}`.
  - **Editor UI:** create, duplicate a built-in, reorder sections, preview against a chosen past meeting, and import/export JSON.
  - **Built-in library (30):**
    - General / Auto
    - 1:1, Stand-up, Weekly team sync, Project kick-off, Sprint planning, Retrospective, Design critique, Brainstorm, All-hands
    - Board meeting, Investor update, Investor pitch (VC side)
    - Sales discovery, Sales demo, Pipeline review, Customer check-in / QBR, Customer support call
    - User interview, Candidate interview, Interview debrief
    - Lecture / class, Webinar, Podcast
    - Coaching session, Therapy/SOAP (keep the existing one), Legal consultation
    - Partnership call, Vendor evaluation
    - **Circleback style** (Action Items at top, Overview of 3–6 bullets, then topic sections)
  - **Default template** setting.
  - **Auto selection:** rules (title regex, attendee domains, external/internal, attendee count) first; otherwise LLM classification (`tagging` route) over the template descriptions.
  - **`/template` slash command** in the notepad, before or during a meeting, sets the meeting's template and inserts its section headings as a scaffold.
  - **Switch template after notes exist:** the dropdown on the Enhanced tab regenerates.
- **Accept:**
  - [ ] All 30 built-ins validate.
  - [ ] Auto-selection picks "Sales discovery" for the external sales fixture and "1:1" for the 1:1 fixture.

#### NOT-05 · User profile and global AI context
- **Wave** 2 · **Depends on** FND-06 · **Size** S
- **Owns:** `settings.profile`
- **Build:**
  - Settings → Profile: name, email(s) (used for the Me person and to tell internal from external by domain), role, company, focus areas, writing-style notes, and glossary/context (free text).
  - Expose it to every prompt through the gateway's system-context helper.
- **Accept:**
  - [ ] Changing the role changes the tone of the notes in a mock-provider prompt snapshot test.

#### NOT-06 · Edit notes by asking
- **Wave** 5 · **Depends on** NOT-02, AI-02 · **Size** S
- **Owns:** `notes/edit/`, chat tool `edit_notes`
- **Parity:** G §3.3, C §9.2 (write actions)
- **Build:**
  - In meeting chat, requests such as "make this more detailed", "Niall is spelled Nyal" or "remove the pricing section" produce a **diff preview** (side by side or inline) with Accept/Reject.
  - When text is selected, the edit is scoped to the selection.
  - Uses the `notes.edit` route with the current `EnhancedDoc` JSON. Origin markers are preserved.
- **Accept:**
  - [ ] Rename-everywhere requests update every occurrence.
  - [ ] Undo works.

#### NOT-07 · Suggested follow-up emails
- **Wave** 5 · **Depends on** NOT-02, ORG-01, INT-07 (optional for sending) · **Size** M
- **Owns:** pipeline stage `follow_up_draft`, `features/followups/`
- **Parity:** G §7.2, C §4 (draft and send)
- **Build:**
  - **When:** after a meeting with at least one external attendee and 10 or fewer people.
  - **Decide first:** an LLM decides whether a follow-up is warranted. It stays quiet when a colleague owns the follow-up or the conversation moved to Slack.
  - **Draft:** subject and body with highlighted `[placeholders]`.
  - **Where it appears:** a "Suggested follow-ups" section on Home for 3 days, and a "View draft" banner on the meeting.
  - **Editor:** To/CC/BCC (pre-filled with external attendees).
  - **Send options:**
    - **Send via Gmail or Outlook** (INT-07) with a 10 s undo.
    - **Open in Mail.app** (mailto) or **Copy** when no email account is connected.
  - Dismiss is permanent.
- **Accept:**
  - [ ] The draft is generated for the external sales fixture and not for the internal standup.

#### NOT-08 · Pre-meeting briefs
- **Wave** 5 · **Depends on** ORG-04, AI-02, ORG-01 · **Size** M
- **Owns:** `briefs/`, the `brief` job (scheduled)
- **Parity:** G §7.1, C §14 (person/company summaries)
- **Build:**
  - **Schedule:** a background job runs at 06:00 local time and again 60 minutes before each event.
  - **Which meetings:** upcoming events with external attendees, or (a setting) all events with at least 2 attendees.
  - **Sources:**
    - previous meetings with the same people or company,
    - open action items involving them,
    - last meeting in the series,
    - the person/company summaries,
    - the event description,
    - optionally the web (the chat agent's web tool) and email (INT-07).
  - **Output:** 2–4 bullets, each with a citation. **No brief** if there is too little information.
  - **Where it shows:** at the top of the upcoming meeting's note and in the 1-minute-before notification. It collapses once read. Thumbs down to give feedback.
- **Accept:**
  - [ ] A brief appears for an upcoming fixture event whose attendees have past meetings.
  - [ ] No brief appears for a new contact.

#### NOT-09 · Sensitive-info redaction
- **Wave** 6 · **Depends on** NOT-02 · **Size** S
- **Build:**
  - Optional setting: before notes are generated and exported, redact passwords, API keys, credit-card numbers (Luhn check), SSNs and one-time codes found in the transcript.
  - Uses regex plus a light LLM check. Redacted spans are shown as `[redacted]`, and the originals stay in the transcript unless the user deletes them.
- **Accept:**
  - [ ] Fixture secrets never reach the enhanced notes or exports.

### 6.4 Phase 3: Organization and entities

#### ORG-01 · Calendar sync
- **Wave** 2 · **Depends on** FND-07, FND-06 · **Size** L
- **Owns:** `calendar/` (new), the `calendar_*` tables
- **Parity:** G §1 (calendars), C §3.1
- **Build:**
  - **Apple Calendar via EventKit (default)** through `objc2-event-kit`. Request full access; this covers every account the user added to macOS Calendar (Google, Exchange, iCloud) with **no OAuth**.
  - **Optional direct connectors** using INT-00 OAuth: **Google Calendar API** and **Microsoft Graph**, for richer attendee and RSVP data and conference links.
  - **Sync window:** past 30 days plus the next 14 days. Refresh every 2 minutes and on wake.
  - **Parse the conference URL** (Zoom, Meet, Teams, Webex, others) from the location, description or conference data.
  - **External detection:** an event is external if any attendee domain is not in the profile's domains.
  - Hide declined events and OOO/Focus/working-location entries.
  - Settings → Calendars: choose which calendars are visible.
  - Upsert people and companies from attendees (ORG-04 owns the entity logic; call its repository).
- **Accept:**
  - [ ] Events from an iCloud and a Google account both appear through EventKit.
  - [ ] Recurring events carry a `series_id`.

#### ORG-02 · Home, Coming up, notifications, auto-record rules
- **Wave** 3 · **Depends on** ORG-01, CAP-06, CAP-10, FND-10 · **Size** M
- **Owns:** `app/page.tsx` (Home), `calendar/notify.rs`, `calendar/rules.rs`
- **Parity:** G §2.2–§2.3, G §3.1 (Home), C §3.2 (joining preferences, recurring "all or just this one"), C §3.5
- **Build:**
  - **Home:**
    - **Coming up** (today and the next 14 days): each event shows a Join button (opens the conference URL **and** starts recording) and a record toggle.
    - Context menu: Open note, Email invitees (mailto), Open in Calendar, and **Review last meeting** (opens the previous meeting with the same attendees).
    - **Recent notes** list.
    - **Suggested follow-ups** (NOT-07).
    - **Open action items** due soon (AI-05).
  - **Notifications:** 1 minute before each event with at least 2 attendees, a custom notification window with **Join & record** and **Record only**. Set it per calendar.
  - **Auto-record rules:**
    - Global settings: record internal meetings? external? meetings organized by others? meetings I haven't accepted?
    - Per-event override, and per-series override with an "all occurrences or just this one" choice.
    - Matching events auto-start recording when the meeting app starts using the mic (with CAP-06), or at start time if the user has the event's note open.
  - **Opening an event:** the event note shows the brief (NOT-08), the notepad and the attendees.
- **Accept:**
  - [ ] The rule matrix is unit tested.
  - [ ] Join & record starts capture within 2 s.

#### ORG-03 · Event linking, series, related meetings
- **Wave** 3 · **Depends on** ORG-01 · **Size** S
- **Owns:** `calendar/linking.rs`, the meeting header event picker
- **Parity:** G §3.1 (event popover, suggestions from the past 24 h), C §3.5 (change the event mid-recording), G §6.5 (related meetings)
- **Build:**
  - When recording starts, auto-link the event nearest to now, overlapping or within 15 minutes.
  - **Event picker** in the meeting header (including mid-recording), suggesting events from the past 24 h. Linking pulls in the title, attendees and description, and creates attendee rows.
  - **Related meetings:** link by `series_id`, or an exact title plus the same attendee set. Show "Previous in series" on the meeting page.
- **Accept:**
  - [ ] Re-linking updates attendees and offers a regenerate prompt.

#### ORG-04 · People and Companies
- **Wave** 3 · **Depends on** FND-07, ORG-01 · **Size** L
- **Owns:** `entities/` (new), `app/people/`, `app/companies/`, pipeline stage `update_entities`
- **Parity:** G §6.4, C §14 (Spaces, AI summaries, pending actions, upcoming events, external links)
- **Build:**
  - **Upsert rules:** people from calendar attendees, speakers and @mentions, keyed by email when there is one. Companies from email domains, skipping public domains (gmail.com, outlook.com and a list of about 50 others).
  - **Manual changes:** add, edit, **merge** people and companies, and set the logo or avatar (fetch the favicon `https://<domain>/favicon.ico` once, then cache it locally; Gravatar for people is optional and off by default).
  - **Person page:** header (name, title, company, email); **AI summary** of who they are, what was discussed and where things stand (stage `update_entities`, refreshed after each meeting with them, `entity.summary` route); pending action items; meetings (most recent first); upcoming events; voice profiles (CAP-04); notes field; **scoped chat** (AI-02).
  - **Company page:** the same, plus its people.
  - **Hover card** on attendee chips and @mentions: recent meetings and quick actions.
  - Lists with search and sorting by last-met date.
- **Accept:**
  - [ ] Fixtures produce the expected people and companies.
  - [ ] Merging moves all links.
  - [ ] Summaries refresh after a new meeting.

#### ORG-05 · Folders
- **Wave** 2 · **Depends on** FND-07, FND-10 · **Size** M
- **Owns:** `features/folders/`, `app/folders/[id]/`, pipeline stage `auto_folder`
- **Parity:** G §6.2–§6.3 (nesting, multi-folder, auto-add recurring, description for chat)
- **Build:**
  - Sidebar tree with nesting to any depth; drag and drop notes and folders; a note can belong to many folders; folder name, icon and description.
  - **Auto-add rules:**
    - When a note from a recurring series is added, prompt "Auto-add future meetings in this series?".
    - Rules can also match a tag or attendee domain.
  - Folder page lists its meetings, with folder-scoped chat (AI-02) and a folder integrations button (AUT-01 shortcut).
  - An "Unfiled" smart folder.
- **Accept:**
  - [ ] A recurring rule files the next fixture instance automatically.
  - [ ] Moving nested folders keeps notes.

#### ORG-06 · Tags and AI auto-tagging
- **Wave** 2 · **Depends on** FND-07, FND-09 · **Size** S
- **Owns:** `features/tags/`, pipeline stage `auto_tag`
- **Parity:** C §14 (tags with descriptions, auto-tagging from descriptions and recent tagged examples, existing tags only, skip if manually tagged)
- **Build:**
  - Tag CRUD with colour and description.
  - Add tags from the meeting header, the list (bulk) and upcoming events (pre-tag).
  - **Auto-tag stage:** a classifier prompt using tag descriptions plus up to 3 recent example titles and summaries per tag. It only uses existing tags and is skipped if the user already tagged the meeting.
  - Settings toggle.
  - Tag changes emit `meeting-tagged` so automations can trigger on them.
- **Accept:**
  - [ ] The auto-tagger test with the mock provider passes.
  - [ ] Manual tags block auto-tagging.

#### ORG-07 · Meetings list, views, bulk actions, trash, merge
- **Wave** 3 · **Depends on** ORG-04, ORG-05, ORG-06 · **Size** M
- **Owns:** `app/meetings/`, `features/meetings-list/`
- **Parity:** C §14 (filters, saved views, bulk, keyboard), G §3.6 (trash), CM (#393 merge)
- **Build:**
  - **List:** paginated (cursor), grouped by day; avatars, tags and duration on each row.
  - **Filters:** date range, people, companies, tags, folders, has action items, external/internal. Filters persist; `F` opens filter search.
  - **Saved Views** as tabs.
  - **Bulk:** click, Shift-click a range, ⌘A for all matching. Actions: tag, add to folder, export, run automation, delete.
  - **Trash:** soft delete with `deleted_at`; Trash view; restore; permanent delete; auto-purge after 30 days, which also deletes the audio.
  - **Merge** 2 or more meetings: concatenate segments with time offsets, union attendees, then regenerate.
- **Accept:**
  - [ ] Filtering 2,000 seeded meetings takes under 150 ms.
  - [ ] Restoring from trash restores everything.

#### ORG-08 · Global search and ⌘K
- **Wave** 3 · **Depends on** FND-07, FND-10 · **Size** M
- **Owns:** the `search_fts` table, `search/` (new), pipeline stage `index` (FTS part), the ⌘K search provider
- **Parity:** C §9.1 (domains, @filters, moments, quick navigation), G §6.5
- **Build:**
  - FTS5 index as described in §4.4, with triggers or stage-based updates and a `search_reindex_all` command.
  - **⌘K:** typing searches meetings, moments (transcript hits with timestamps), people, companies, action items and folders. `@` adds filters (people, company, tag, folder, meeting). A typed domain such as `acme.com` filters by attendee domain. Navigation commands also appear (Settings, New note, Calendar…).
  - **Results page** with Meetings/Moments/People tabs. A moment hit opens the meeting at that timestamp.
  - Hybrid ranking with AI-01 once it exists, behind a feature check.
- **Accept:**
  - [ ] Search over 2,000 seeded meetings returns in under 100 ms.
  - [ ] Snippets show highlighted matches.

#### ORG-09 · Stats and year in review
- **Wave** 6 · **Depends on** ORG-04 · **Size** S
- **Parity:** C §15 (usage dashboard and heatmap), G §3.7 (Crunched)
- **Build:**
  - Stats page: number of meetings, hours, unique people, internal vs external split, a GitHub-style heatmap, top people, busiest days, LLM usage and cost (from `llm_calls`).
  - "Year in review", a generated recap available in December.
- **Accept:**
  - [ ] Numbers match the fixture data.

### 6.5 Phase 4: AI

#### AI-01 · Embeddings and semantic index
- **Wave** 3 · **Depends on** FND-09, FND-08, ORG-08 · **Size** M
- **Owns:** the `embeddings` table, `search/semantic.rs`, pipeline stage `index` (vector part)
- **Build:**
  - **Vector store:** load the `sqlite-vec` extension (pinned, bundled). If loading it is not viable, use an in-memory cosine index persisted as BLOBs; personal scale is fine.
  - **Embedding:** route `embed`, defaulting to local `nomic-embed-text-v1.5` Q8 GGUF through llama-helper `Embed`, with the `search_document:` and `search_query:` prefixes.
  - **Chunking** as in §4.4.
  - Backfill job for existing meetings, with a progress UI in Settings → AI → Index.
  - `search_hybrid(query, filters, k)`.
- **Accept:**
  - [ ] A paraphrased query finds the right fixture meeting in the top 3 for 8 out of 10 test questions (store them as a test).

#### AI-02 · Chat agent ("Ask Deino")
- **Wave** 4 · **Depends on** AI-01, FND-09, ORG-04, ORG-05 · **Size** XL (02a backend agent, 02b UI)
- **Owns:** `chat/` (new), the `chat_threads`/`chat_messages` tables, `features/chat/`
- **Parity:** G §5 (scopes, agentic retrieval, coverage notes, citations, ⌘J, dictation, model picker), C §9.2–§9.3 (@filters, history, streaming, calendar questions, per-meeting history)
- **Build (02a, backend):**
  - **Agent loop** on `Gateway.with_tools` (route `chat`), streaming.
  - **Read tools:**
    - `search(query, filters)` (hybrid)
    - `list_meetings(filters)`
    - `read_meeting_notes(ids)`
    - `read_transcript(id, from_ms?, to_ms?)`
    - `list_action_items(filters)`
    - `get_person` / `get_company`
    - `calendar_lookup(range, attendee?)`
    - `web_search(query)`: optional. Use Anthropic's server-side web search tool when the route is Claude; otherwise a user-provided Brave or Tavily key. Off by default.
    - `read_attachment(id)`
  - **Scope:** current meeting / folder / person / company / tag / multi-selection / all. It is injected as default filters, and the model can widen scope only when asked.
  - **Strategy:** prefer notes over transcripts (Granola's agent design, G §5.2). Budget-aware.
  - **Citations:** answers carry markers `[[cite:meeting_id:segment_id|ms]]`, and the UI renders them as chips.
  - **Coverage:** every answer returns a coverage list of the meetings it read.
  - **During a meeting:** the agent sees the live transcript so far ("what did I miss?", "what should I ask next?").
  - **Persistence:** threads are saved per scope. Per-meeting history plus a global history with search.
  - **Attachments:** PDF, DOCX, TXT and images (text extracted) as context.
- **Build (02b, UI):**
  - A floating chat bar ("Ask anything", ⌘J) on every page, scoped to the current view and showing a scope chip that can be changed.
  - Right-panel chat on the meeting page.
  - A full-page Chat view with history.
  - A model picker (routes or models).
  - Citation chips: hover shows a preview; click opens the meeting at that timestamp.
  - A "Thought for Xs" disclosure for thinking models.
  - Copy without citation markers.
  - Mic dictation into the chat box, reusing the STT engine.
- **Accept:**
  - [ ] A 20-question eval set over the fixtures scores at least 80% correct with valid citations (Claude route); store the eval.
  - [ ] Streaming first token arrives in under 2 s.
  - [ ] Scope is respected (test).

#### AI-03 · Chat write actions
- **Wave** 5 · **Depends on** AI-02, AI-05, ORG-06 · **Size** M
- **Owns:** `chat/tools_write.rs`, `features/chat/cards/`
- **Parity:** C §9.2 (write actions, Oct 2026), G §5.4 (confirm cards)
- **Build:**
  - **Tools:**
    - `create_action_item`, `update_action_item` (status, assignee, due), `delete_action_item`
    - `add_tag`, `remove_tag`, `create_tag`
    - `rename_meeting`
    - `add_to_folder`
    - `set_private_notes(upcoming_event_id, md)`
    - `edit_notes` (NOT-06)
    - `draft_email` (NOT-07/INT-07)
    - `post_to_slack` (INT-01)
    - `create_calendar_event` (Google/Graph only; shown disabled otherwise)
  - **Every write renders a confirmation card** (editable fields, Confirm/Cancel) and nothing executes without a click. Show conflict warnings for calendar events.
- **Accept:**
  - [ ] Writes never execute without confirmation (test).
  - [ ] Undo works for local entities.

#### AI-04 · Recipes
- **Wave** 5 · **Depends on** AI-02 · **Size** M
- **Owns:** the `recipes` table, `features/recipes/`, `app/recipes/`
- **Parity:** G §5.3
- **Build:**
  - **Recipe fields:** `{name (= /slug), prompt, type: single|multi, model_route?, description}`.
  - Typing `/` in the chat bar lists recipes valid for the current scope.
  - **Recipes page:** Built-in / Mine; search over name and prompt; **Preview** runs the recipe on a chosen meeting or folder; **Remix** (duplicate and edit); export/import JSON or a share link string (`deino://recipe?data=<base64>`).
  - **Built-in library (20):**
    - Coach me, Prep me for this person, Write a brief, Write follow-up email
    - Summarize for my manager, Extract decisions, List open questions
    - Customer pain points, Feature requests, Competitor mentions, Objections & responses
    - Hiring scorecard, PRD draft from discussion, Weekly recap
    - How have I changed, How I like to work, Prep my day
    - Mochary-style 1:1 review, Risks & blockers, What did I commit to?
  - Recipes chain inside a thread.
- **Accept:**
  - [ ] Single-meeting recipes are hidden in multi scopes and the other way round.
  - [ ] Preview works.

#### AI-05 · Action items system
- **Wave** 3 · **Depends on** FND-07, FND-09, ORG-04 · **Size** L
- **Owns:** `action_items/` (new), `app/action-items/`, pipeline stage `extract_action_items`
- **Parity:** C §8 (auto-assign, title and description, global list, filters, views, bulk, mark all mine done, standalone), G §3.5; CM §5 (LokalBot "Today" digest)
- **Build:**
  - **Extraction stage** (`extract.action_items`, structured):
    - Fields: `{title, description (1–2 sentences of context), assignee (resolve to an attendee or speaker, or Me), due_at (resolve relative dates like "next Friday" against the meeting date), priority?, evidence_segment_id}`.
    - Only clear commitments.
    - Make it idempotent: regenerating replaces only AI items that haven't been touched.
  - **Notes sync:** action items render at the top of enhanced notes and as todo blocks. Ticking either one syncs both.
  - **Action Items page:**
    - Default filter is Mine/open.
    - Filters: assignee scope (Me / Others / person / Unassigned / Anyone), status, tag, company, folder, due (overdue/today/this week), date range.
    - Saved views; bulk (status, reassign, due date, delete); "Mark all mine done".
    - Click through to the source moment.
  - **Standalone items:** create from the page or ⌘K.
  - **Reminders:** a macOS notification at 9:00 for items due today, and a "Today" digest on Home.
  - Emit `action-item-*` events for INT-03 sync and AUT-01.
- **Accept:**
  - [ ] Extraction precision is at least 0.8 on the labelled fixture set (store the labels).
  - [ ] Ticking an item in notes updates the page.

#### AI-06 · Custom AI insights
- **Wave** 4 · **Depends on** FND-08, FND-09 · **Size** M
- **Owns:** the `insight_defs`/`insight_results` tables, `insights/`, `app/insights/`, pipeline stage `insights`
- **Parity:** C §10.3 (full spec, including output shape and template gallery)
- **Build:**
  - **Insight definition:** `{name, description(prompt), fields:[{name, type: text|number|boolean|enum|date, enum_values?}], cardinality: auto|N, apply_when: conditions (same schema as AUT-01) | manual}`.
  - **Template gallery (12):**
    - Discovery scorecard (questions, pain, budget, timeline, decision-maker, 1–5 score)
    - Customer feedback (feature requests, complaints, praise, quotes)
    - Competitor mentions
    - Health signals (Red/Yellow/Green)
    - Executive summary
    - Interview debrief
    - Questions asked
    - Objections
    - Risks
    - Decisions
    - Follow-up email
    - Deal summary for CRM
  - **Output:** `[{value: string | {field: value}, speaker, timestamp_ms}]`.
  - **Display:** an Insights tab per meeting, and a global Insights page with one table per insight across meetings (sortable columns, expand cells, **CSV export**).
  - "Run on past meetings" with selection.
- **Accept:**
  - [ ] Typed fields are validated.
  - [ ] Speaker and timestamp attribution link to the transcript.

#### AI-07 · Chat connectors (MCP client)
- **Wave** 6 · **Depends on** AI-02, INT-00 · **Size** M
- **Owns:** `chat/connectors/`
- **Parity:** C §9.2 (built-in connectors plus custom MCP)
- **Build:**
  - Settings → Connectors: add any MCP server (stdio command, or Streamable HTTP URL with an OAuth/DCR flow through INT-00, or a bearer token).
  - Tools are discovered and offered to the chat agent, namespaced by server.
  - Per-tool allow, ask or deny. Write-type tools default to **ask**, using the AI-03 confirmation card.
  - Presets with setup instructions for Linear, Notion, Asana, HubSpot, Attio, PostHog, Intercom and Ashby (their official MCP endpoints).
- **Accept:**
  - [ ] Connecting a local test MCP server exposes its tools.
  - [ ] Write tools require confirmation.

#### AI-08 · Weekly roundup
- **Wave** 6 · **Depends on** AI-02, AI-05 · **Size** S
- **Parity:** C §13.1 (weekly roundup)
- **Build:**
  - Every Friday at 16:00 (configurable): a generated digest of the week covering key decisions, themes, people met, open and overdue action items, and next week's important meetings.
  - Shown as a pinned item on Home and sent as a notification. Optionally exported (EXP-02) or emailed (INT-07).
- **Accept:**
  - [ ] Generated correctly over the fixture week.

### 6.6 Phase 5: Automations, export, integrations, developer platform

#### AUT-01 · Automations engine
- **Wave** 4 · **Depends on** FND-08, ORG-06, AI-06, INT-00 · **Size** L
- **Owns:** `automations/` (new), the `automations`/`automation_runs` tables, `app/automations/`, pipeline stage `automations`
- **Parity:** C §10.1–§10.2 (full spec), G §8.3 (folder auto-post); CM §5 (Muesli post-meeting hooks)
- **Build:**
  - **Model:** `{name, enabled, auto_run, condition_groups: [[Condition]] (AND within a group, OR across groups; an empty list runs on every meeting), steps: [Action], created_at}`.
  - **Conditions:**
    - tags include/exclude
    - folder is
    - participants include/exclude
    - title is / is not / contains / does not contain / matches regex
    - invitee emails
    - invitee domains are all / are not all
    - invitee count = / > / <
    - external yes/no
    - template is
    - meeting duration >
    - calendar is
  - **Triggers:** pipeline completion; tag applied (including on upcoming meetings, which arms the automation); manual run.
  - **Actions** (each is a `Step` implementation; integration steps come from INT cards and are registered here):
    - generate insight (AI-06), whose output feeds later steps
    - add tag, add to folder
    - export file (EXP-02)
    - webhook (DEV-01)
    - **run shell script**: JSON payload on stdin, 60 s timeout, working dir is the meeting folder, gated behind a settings toggle
    - Slack (INT-01), Notion (INT-02), Linear/task tools (INT-03), CRMs (INT-04), Google Docs or Apple Notes (INT-05)
    - email draft/send (INT-07)
    - macOS notification
  - **Payload field selection** per step: notes, action items, transcript, insights, attendees, link, audio path.
  - **UI:** list, a builder with condition groups and a step list, "Test on latest meeting", **Duplicate**, a "Matching meetings" tab with **Run on selected**, and a per-meeting ⋯ → Automations menu with run status and Rerun.
  - **Run log** with each step's result and error.
- **Accept:**
  - [ ] Condition logic is unit tested across 30 cases.
  - [ ] A failed step doesn't block later steps unless "stop on failure" is set.
  - [ ] Shell hook test passes.

#### EXP-01 · Exports
- **Wave** 2 · **Depends on** FND-07 · **Size** M
- **Owns:** `export/` (new), the share/export menu on the meeting page
- **Parity:** CM §4.4 (#816; upstream PR #714), G §8.1 (copy as Markdown, email), C §5
- **Build:**
  - **Per meeting:**
    - Markdown, with YAML frontmatter (title, date, attendees, tags, folders, duration, links)
    - PDF (HTML template rendered by headless webview print, or a Rust `printpdf`/`typst` crate; pick one)
    - DOCX (`docx-rs`)
    - TXT
    - JSON (full object)
    - SRT/VTT (transcript)
  - The user chooses sections: notes, my notes, action items, insights, transcript.
  - **Copy as rich text** (HTML on the clipboard) keeps formatting in Gmail, Docs and Slack.
  - Email: `mailto:` with the notes in the body.
- **Accept:**
  - [ ] Golden-file tests for Markdown, TXT, JSON and SRT.
  - [ ] PDF and DOCX open in Preview and Word.

#### EXP-02 · Auto-export and full account export
- **Wave** 3 · **Depends on** EXP-01, FND-08 · **Size** S
- **Owns:** pipeline stage `auto_export`, Settings → Export
- **Parity:** CM §4.4 (#576, #344, #508 Obsidian), G §10.5 (CSV export), C §5 (account export)
- **Build:**
  - **Auto-export** after each meeting to a chosen folder, for example an Obsidian vault.
    - Filename template (`{date} {title}.md`), subfolder template (`{yyyy}/{mm}`), sections, overwrite on regenerate.
    - Optional transcript sidecar file, and links between people pages (`[[Person]]`) when "Obsidian mode" is on.
  - **Full export:** a zip of every meeting (Markdown and JSON), audio optionally, plus CSVs of meetings, action items and insights.
- **Accept:**
  - [ ] The exported vault opens in Obsidian with working wiki links.

#### INT-00 · Integrations framework
- **Wave** 2 · **Depends on** FND-06 · **Size** M
- **Owns:** `integrations/` (core), the `integration_connections` table, Settings → Connections
- **Build:** everything in §4.5:
  - the loopback PKCE OAuth helper (plus a device-code flow helper)
  - token refresh
  - `Destination` and `Step` traits
  - per-provider client ID settings
  - a generic "Share to…" menu on the meeting page that lists connected destinations
  - an outbound HTTP client with retries and a rate limiter
- **Accept:**
  - [ ] A mock OAuth provider in tests completes the flow.
  - [ ] Tokens are stored in the Keychain only.

#### INT-01 · Slack
- **Wave** 4 · **Depends on** INT-00, AUT-01 · **Size** S
- **Parity:** G §9 (Slack row), C §10.2 (thread format, outcome toggles)
- **Build:**
  - OAuth to a user-created Slack app (scopes `chat:write`, `channels:read`, `groups:read`, `users:read`).
  - **Share to Slack** from a meeting: a channel picker and outcome toggles. The post is a thread: the parent message has the title and summary, replies carry details and action items.
  - An automation step, and DMs to yourself.
- **Accept:**
  - [ ] Payload snapshot tests.
  - [ ] Manual QA posts to a test workspace.

#### INT-02 · Notion
- **Wave** 4 · **Depends on** INT-00, AUT-01 · **Size** M
- **Parity:** G §9 (Notion row), C §10.2 (page vs database, property mapping, one row per action item)
- **Build:**
  - OAuth to a public Notion integration, or paste an internal token.
  - **Destination page:** create a child page with the chosen outcomes, converting Markdown to blocks.
  - **Destination database:** map properties to fields (title, date, attendees, tags, summary, a custom insight field). Multi-valued outcomes produce one row per item.
  - Share-to and automation step. Update the existing page on regenerate when that option is on.
- **Accept:**
  - [ ] Tests for property mapping.
  - [ ] Manual QA.

#### INT-03 · Task tools with two-way sync
- **Wave** 4 · **Depends on** INT-00, AI-05, AUT-01 · **Size** L (split per provider if parallel)
- **Parity:** C §8 (Linear assignment matching, monday, HubSpot/Attio tasks); better than both: two-way status
- **Build:**
  - A `TaskSink` trait: `create(item) -> external_ref`, `update`, and `poll_status(refs)`.
  - **Providers:**
    - **Linear** (OAuth or API key): team picker, assignee matched by email, assigned/unassigned toggles.
    - **Todoist**, **Asana**, **Jira Cloud**, **monday.com**, **ClickUp**, **Trello** (API keys or OAuth).
    - **Apple Reminders** through EventKit (local; best default).
    - **Things 3** through its URL scheme (create only).
  - **Two-way:** poll status every 10 minutes. Completing the task externally marks the action item done, and the other way round.
  - Automation step "Send action items to <tool>", plus a per-item "Send to…" action.
- **Accept:**
  - [ ] Reminders round-trip works.
  - [ ] Linear passes recorded-fixture tests.
  - [ ] No duplicate creation on re-run.

#### INT-04 · CRMs
- **Wave** 5 · **Depends on** INT-00, AUT-01, ORG-04 · **Size** L (split per provider)
- **Parity:** G §9 (HubSpot, Attio, Affinity rows), C §10.2 (HubSpot meeting engagement and tasks, Salesforce Event, Attio note and tasks, Zoho note)
- **Build:**
  - Match records by attendee email (contact, then company or deal).
  - **Per CRM:**
    - **HubSpot:** a logged Meeting engagement with the outcomes, and tasks for action items.
    - **Salesforce:** an Event on the contact or opportunity.
    - **Attio:** a note on the person, company or deal, plus tasks.
    - **Affinity:** a note (API key).
    - **Zoho CRM:** a note on the contact.
  - A review dialog for manual pushes (suggested records, which can be removed or searched); automations push silently.
  - Store external record links on people and companies (`external_refs_json`) and show them on entity pages.
- **Accept:**
  - [ ] Recorded-fixture tests for each CRM.
  - [ ] Idempotent: no duplicates on re-run.

#### INT-05 · Documents: Google Docs and Apple Notes
- **Wave** 5 · **Depends on** INT-00, AUT-01 · **Size** S
- **Build:**
  - **Google Docs:** create a doc in a chosen Drive folder per meeting (OAuth `drive.file`).
  - **Apple Notes:** create a note in a chosen folder through AppleScript/JXA.
- **Accept:**
  - [ ] Manual QA for both.

#### INT-07 · Email: Gmail and Outlook (context, drafts, send)
- **Wave** 4 · **Depends on** INT-00 · **Size** M
- **Parity:** G §7.2, G §5.4 (Gmail draft card, labels, save as draft), C §4 (email context on demand, not stored; reply in thread)
- **Build:**
  - Gmail API (OAuth `gmail.readonly` + `gmail.compose` + `gmail.send`) and Microsoft Graph Mail.
  - **Context:** fetch recent threads with the meeting's attendees **on demand** (not stored) for notes, briefs and chat (`email_search` tool).
  - **Draft and send:** the follow-up editor, "Save as draft" (with a Gmail label), Send with a 10 s undo, and reply in an existing thread when one is found.
- **Accept:**
  - [ ] No email bodies persisted (test).
  - [ ] Manual QA sends to self.

#### DEV-01 · Outbound webhooks (and Zapier/Make)
- **Wave** 3 · **Depends on** FND-08, INT-00 · **Size** S
- **Owns:** the `webhooks`/`webhook_deliveries` tables, `webhooks/`
- **Parity:** G §9.3 (Standard Webhooks, retries, auto-disable), C §12.4 (full payload, `x-signature` HMAC, field selection, test send)
- **Build:**
  - **Endpoints:** URL, events, field selection, and a signing secret shown once.
  - **Events:**
    - `meeting.ready`
    - `notes.generated`, `notes.edited`
    - `action_item.created`, `action_item.updated`
    - `insight.generated`
  - **Payload:** the full Circleback-style meeting payload with the selected fields.
  - **Signing:** both a Standard-Webhooks signature (`webhook-id`, `webhook-timestamp`, `webhook-signature`) **and** `x-signature` (hex HMAC of the body).
  - **Delivery:** 15 s timeout; retry with exponential backoff for 24 h while the app is running; delivery log; "Send test" with the latest meeting.
  - A setup doc for Zapier ("Catch Hook") and Make ("Custom webhook").
- **Accept:**
  - [ ] Signature verification test vectors pass.
  - [ ] Retries are observed in tests.

#### DEV-02 · Local MCP server
- **Wave** 4 · **Depends on** AI-02, AI-05, ORG-04, ORG-01 · **Size** M
- **Owns:** `mcp_server/` (new)
- **Parity:** G §9.1 (tools), C §12.2 (tools, including write); CM (Meetily PRO 1.11 MCP)
- **Build:**
  - **Transports:**
    - Streamable HTTP on `127.0.0.1:<port>` with a bearer token from Settings.
    - A stdio shim binary (`deino-mcp`) that proxies to the running app, for Claude Desktop and Claude Code.
  - Use the `rmcp` crate.
  - **Read tools:**
    - `search_meetings`, `read_meetings` (notes, my notes, attendees, action items, insights, tags, audio path), `get_transcript`, `search_transcripts`
    - `list_folders`, `list_tags`
    - `search_action_items`
    - `search_calendar_events`
    - `find_people`, `find_companies`
    - `ask_deino` (runs the chat agent and returns an answer with citations)
  - **Write tools** (each one toggleable in Settings, off by default):
    - `create_action_item`, `update_action_item`
    - `update_meeting` (title, notes, tags)
    - `set_upcoming_private_notes`
    - `start_recording`, `stop_recording`
  - **Settings:** a copy-paste snippet for `claude mcp add deino --transport http http://127.0.0.1:<port>/mcp --header "Authorization: Bearer …"` and for Claude Desktop config.
- **Accept:**
  - [ ] MCP Inspector lists the tools.
  - [ ] Claude Code can answer "what did I commit to last week?" end to end (manual QA).

#### DEV-03 · Local REST API
- **Wave** 4 · **Depends on** ORG-07, AI-05, ORG-04 · **Size** M
- **Owns:** `api_server/` (new), the `api_keys` table
- **Parity:** C §12.1 (endpoints, cursor pagination, import endpoint), G §9.2
- **Build:**
  - An `axum` server on `127.0.0.1` (off by default).
  - **API keys:** read-only or read-write, hashed with argon2, created in Settings and shown once.
  - **Endpoints (`/v1`):**
    - `GET /meetings` (filters, cursor), `GET /meetings/{id}` (include=transcript)
    - `PUT /meetings/{id}`, `DELETE /meetings/{id}` (to trash)
    - `POST /meetings` (import with transcript JSON, as in C §12.1)
    - `GET /search`
    - `GET/POST/PUT/DELETE /action-items`
    - `GET /calendar/events`, `PUT /calendar/events/{id}/notes`
    - `GET /people`, `GET /companies`
    - `GET/POST/PUT/DELETE /tags`
    - `GET /folders`
    - `POST /recording/start`, `POST /recording/stop`
  - An OpenAPI 3.1 document served at `/v1/openapi.json` (`utoipa`).
  - Rate-limit at 20 requests per second.
- **Accept:**
  - [ ] Integration tests for every endpoint.
  - [ ] OpenAPI validates.

#### DEV-04 · CLI
- **Wave** 5 · **Depends on** DEV-03 · **Size** S
- **Owns:** a `cli/` crate (binary `deino`)
- **Parity:** C §12.3
- **Build:**
  - A Rust CLI (`clap`) that talks to the local API. It reads the key from the `DEINO_API_KEY` env var or `deino auth login --api-key`.
  - **Commands:**
    - `deino meetings [search] --last N --from --to --tag --person`
    - `deino meetings read <ids>`, `deino meetings update`
    - `deino transcripts <search>`, `deino transcripts read`
    - `deino action-items [--status] create|update|delete`
    - `deino calendar [--from --to] update <id> --private-notes`
    - `deino people`, `deino companies`, `deino tags`
    - `deino export <id> --format md|pdf|json`
    - `deino record start|stop`
  - `--json` everywhere.
  - "Install CLI" in Settings symlinks it into `/usr/local/bin`, with confirmation.
- **Accept:**
  - [ ] An end-to-end test against the API runs in CI.

#### DEV-05 · Deep links, global hotkeys, Raycast, Shortcuts
- **Wave** 4 · **Depends on** TRN-04, FND-10 · **Size** S
- **Parity:** C (Raycast, deep links), CM (#260 hotkeys; upstream PR #732)
- **Build:**
  - **`deino://` scheme** (`tauri-plugin-deep-link`):
    - `meeting/<id>[?t=ms]`
    - `new-note[?event=<id>]`
    - `record/start`, `record/stop`
    - `search?q=`
    - `recipe?data=`
  - **Global hotkeys** (`tauri-plugin-global-shortcut`, configurable): start/stop recording, quick note, open chat.
  - Document Raycast script commands and an Apple Shortcuts recipe that use the deep links and CLI. No native App Intents.
- **Accept:**
  - [ ] Every link route is tested.
  - [ ] Hotkeys work while the app is in the background.

### 6.7 Privacy and data controls

#### PRV-01 · Retention policies
- **Wave** 3 · **Depends on** FND-08, ORG-07 · **Size** S
- **Parity:** G §10.2, C §15 (recording, transcript and all-data sliders)
- **Build:**
  - Settings → Privacy, three separate retention settings:
    - **Audio/video:** Never / 7 / 30 / 90 / 365 days.
    - **Transcript:** must be kept at least as long as audio.
    - **Everything.**
  - A daily job deletes expired items. Notes stay when only the transcript is deleted; chat then answers from notes and the UI shows "transcript deleted".
  - Show a "pause for sensitive discussion" tip in the record panel.
- **Accept:**
  - [ ] Simulated-clock tests.
  - [ ] Regenerating is disabled when the transcript is gone.

#### PRV-02 · Encryption at rest and app lock
- **Wave** 6 · **Depends on** FND-07 · **Size** M
- **Build:**
  - Optional SQLCipher: sqlx with the `sqlcipher` feature, key in the Keychain, migration of an existing DB.
  - Optional Touch ID / password app lock after N minutes idle (LocalAuthentication).
  - Audio files stay plaintext but can be moved to an encrypted APFS volume (documented).
- **Accept:**
  - [ ] An encrypted DB can't be opened without the key.
  - [ ] Migration is lossless.

### 6.8 Removed cards

The cloud cards (CLD-01 to CLD-07: hosted backend, share links, email recaps, sync, mobile apps, meeting bot, Slack bot) and the team cards (TEAM-01, TEAM-02) were removed on 2026-10-08. The owner wants a Mac app with no cloud services and no meeting bot, and the phone app is a future consideration (§10). See §5.8 for the local alternatives. Do not build them.

---

## 7. Wave schedule and dependency graph

Run the waves in order. Cards within a wave can run in parallel. Tick a card when it is merged.

| Wave | Cards (✅ = merged) | Notes |
|---|---|---|
| **0** | FND-01 → FND-02 → FND-10a → FND-13 → (FND-03 ∥ FND-04) | One after another. These reshape the whole repo. |
| **1** | FND-05 ∥ FND-06 ∥ FND-07 ∥ FND-10 ∥ CAP-01; then FND-09 and FND-12 (after FND-06) | FND-05 and FND-07 coordinate on `transcript_segments`. Land FND-07's migration first and FND-05 rebases onto it. |
| **2** | FND-08, then in parallel: CAP-02 ∥ CAP-06 ∥ CAP-10 ∥ NOT-01 ∥ NOT-04 ∥ NOT-05 ∥ ORG-01 ∥ ORG-05 ∥ ORG-06 ∥ EXP-01 ∥ INT-00 | FND-08 merges first, because many cards add stages. |
| **3** | CAP-03 ∥ CAP-07 ∥ CAP-08 ∥ CAP-11 ∥ TRN-01 ∥ TRN-05 ∥ NOT-02 ∥ ORG-02 ∥ ORG-03 ∥ ORG-04 ∥ ORG-08 ∥ AI-05 ∥ EXP-02 ∥ DEV-01 ∥ PRV-01; then TRN-03 (after TRN-01), ORG-07 (after ORG-04), AI-01 (after ORG-08) | The heaviest wave. NOT-02 is on the critical path. |
| **4** | CAP-04 ∥ CAP-09 ∥ TRN-02 ∥ TRN-04 ∥ NOT-03 ∥ AI-02 ∥ AI-06 ∥ AUT-01 ∥ INT-07 ∥ DEV-02 ∥ DEV-03 ∥ DEV-05 ∥ FND-11; then INT-01 ∥ INT-02 ∥ INT-03 (after AUT-01) | |
| **5** | CAP-05 ∥ CAP-12 ∥ TRN-06 ∥ NOT-06 ∥ NOT-07 ∥ NOT-08 ∥ AI-03 ∥ AI-04 ∥ INT-04 ∥ INT-05 ∥ DEV-04 | |
| **6** | CAP-13 ∥ NOT-09 ∥ ORG-09 ∥ AI-07 ∥ AI-08 ∥ PRV-02 | At this point **Deino matches Granola and Circleback on every local feature.** |

### 7.0 Execution notes (speed-ups, 2026-10-09)

- **Parallelism cap:** keep about 6–7 card agents building at once on the M4 Max. Every shell sources `scripts/dev-env.sh` (sccache + `CARGO_BUILD_JOBS=6`). When one agent finishes, start the next ready card.
- **Port-first unblocking:** a card whose only dependency is an interface may start early by coding against a port trait, with the adapter wired when the dependency merges.
  - **FND-09** (LLM gateway) defines a `CredentialSource` port instead of waiting for FND-06's `deino-secrets`.
  - **FND-12** (net) defines a `ServicePolicy` port for enable and disable.
  - **INT-00** codes against both ports.
- **Split cards:**
  - **FND-05a** · `crates/deino-audio`: AVAssetWriter CMAF track writer, `recording.json` manifest, `truncate_to_playable` recovery and finalize (ported from the spike), plus `MockAudioSource`. No DB writes. It can start now.
  - **FND-05b** · Rust-owned live persistence: create the meeting row at start and stream segments into `transcript_segments`. Needs FND-07 and FND-05a.
  - **CAP-06** lives in its own crate `crates/deino-detect` (heavy macOS native code), so it doesn't wait on FND-05.
  - **EXP-01** pure renderers (Markdown, TXT, JSON, SRT/VTT, DOCX, PDF) work from a DTO and can start before FND-07 merges. Wire them to the repos afterwards.
- **Benchmarks:** take acceptance timings only when the machine load is below about 20, or label them with the load.
- **Never kill processes by pattern.** `pkill -f target/debug/deino` also kills other agents' dev apps. Record the PID you started (`$!`) and kill only that PID.

### 7.1 Progress log

| Card | Status | Merged |
|---|---|---|
| FND-01 Rebrand | ✅ merged | 2026-10-08 |
| FND-02 Cleanup, English-only, inline plugins | ✅ merged | 2026-10-08 |
| FND-10a Vite migration | ✅ merged | 2026-10-08 |
| FND-13 Code-quality baseline and workspace skeleton | ✅ merged | 2026-10-09 |
| FND-04 Test fixtures and mocked-IPC dev mode | ☑️ done on branch `feat/FND-04-fixtures-mock-ipc` | |
| FND-07 Core schema | ☑️ done on branch `feat/FND-07-core-schema` | |
| FND-06 Typed settings store and Keychain secrets | ☑️ done on branch `feat/FND-06-settings-secrets` | |
| FND-05a `crates/deino-audio` crash-safe track writer | ✅ merged (ADR 0014) | 2026-10-09 |
| FND-10 App shell, navigation, design system, dark mode | ✅ merged | 2026-10-09 |
| FND-03 CI and quality gates | ✅ merged | 2026-10-09 |
| CAP-01 Parakeet Unified EN 0.6B engine (default) | ✅ merged (ADR 0016) | 2026-10-09 |
| FND-09 LLM gateway | ✅ merged (ADR 0017) | 2026-10-09 |
| FND-08 Post-meeting pipeline (job runner) | ☑️ done on branch `feat/FND-08-pipeline` (ADR 0018) | |
| FND-12 Network client and data-egress controls | ☑️ done on branch `feat/FND-12-net` (ADR 0019) | |
| CAP-03 Local speaker diarization | ☑️ done on branch `feat/CAP-03-diarization` (ADR 0020; stage `diarize` registered) | |
| NOT-04 Templates v2 | ☑️ done on branch `feat/NOT-04-templates-profile` (ADR 0021; 30 versioned built-ins, auto rules, LLM fallback on `tagging`) | |
| NOT-05 User profile and global AI context | ☑️ done on branch `feat/NOT-04-templates-profile` (gateway `SystemContext`, "Me" person, internal vs external) | |
| NOT-01 Live notepad ("My notes") | ✅ merged 2026-10-09 (ADR 0022; `deino-doc`, `notes` and `people` plugins, BlockNote removed) | |
| CAP-06 Meeting detection and auto-start | ✅ merged 2026-10-09 (ADR 0023; crate `deino-detect`, plugin `detection`) | |
| Spike: native recording (AVAssetWriter CMAF) | ✅ done | `docs/references/spike-native-recording.md` |
| Spike: STT streaming (transcribe-cpp) | ✅ done | `docs/references/spike-stt-streaming.md` |
| Spike: diarization bake-off | ✅ done | `docs/references/spike-diarization.md` |

### 7.2 Follow-ups raised by merged cards (assign to the named card)

| From | Follow-up | Goes to |
|---|---|---|
| FND-01/02 | ~~Two hardware-dependent `device_detection` tests fail on real Macs.~~ Fixed by FND-03: `classify` takes the native answer as an argument (tests pass `None`); the second failure was an f32 rounding bug in `calculate_buffer_timeout` (160 ms became 159.999996 ms on every machine). | ✅ FND-03 |
| FND-02 | ~~Four pre-existing clippy errors in legacy files.~~ Fixed by FND-13. | ✅ FND-13 |
| FND-02 | ~~`whisper_engine.rs` logs transcript text.~~ Fixed by FND-13, together with the same leak in `transcription/worker.rs` and `parakeet_engine.rs`. | ✅ FND-13 |
| FND-02 | Many legacy commands still take a vestigial `auth_token` parameter. Drop it when porting each command. | Each card that ports a legacy command |
| FND-02 | ~~`get_transcription_status` is a stub that always returns zeros.~~ Reports queued chunks or live seconds, processing and idle time (CAP-01). | ✅ CAP-01 |
| FND-02 | ~~`generate_meeting_summary` has too many arguments.~~ Replaced by `deino_core::summaries::SummaryGenerator::generate(&SummaryRequest)` over the gateway; the background job takes one `SummaryJob`. | ✅ FND-09 |
| FND-02 | ~~The `llama-helper` `[profile.release]` is ignored.~~ Moved to `[profile.release.package.llama-helper]` by FND-13; `lto` cannot be set per package, so a fully LTO'd sidecar needs a custom `[profile.sidecar]` and a release-script change. | ✅ FND-13 (LTO: ✅ FND-03, `[profile.sidecar]` used by `release-macos.yml`) |
| FND-02 | Windows MSVC linking after removing the dead `esaxx-rs` patch is unverified. FND-03 added no Windows job: Windows minutes bill at 2x on the private repo, and Linux `cargo check` already covers the non-macOS code paths. | Open: a manual `windows` job, or a check on a Windows machine, when Windows matters |
| FND-01 | `onnxruntime.rs` and `ffmpeg.rs` duplicate the sha256 helper. Moot once FFmpeg is removed (D11). FND-05a added no FFmpeg; `build/ffmpeg.rs` stays while legacy recording, import and re-transcription still call the sidecar. | The last card that removes an FFmpeg caller (FND-05b / CAP-11 / TRN-04) |
| FND-10a | The record screen's permission card still tells macOS users to install BlackHole. Rewrite it for the process tap (§3C). | FND-11 |
| FND-10a | ~~The sidebar has no `shrink-0` and squeezes at narrow widths.~~ The new sidebar is `shrink-0`, resizable (200–320px) and collapsible. | ✅ FND-10 |
| FND-10a | ~~`tauri build` codesign fails in worktrees under `~/Documents` (file attributes).~~ `release-macos.yml` clears extended attributes in a `beforeBundleCommand`, before Tauri signs the bundle. Local `pnpm run tauri:build` under `~/Documents` still needs the same override (see FND-03 below). | ✅ FND-03 |
| FND-10a | ~~Tailwind v4 default palette replaced the v3 hex values.~~ Every palette colour is now a design token; ESLint bans palette classes and hex values. | ✅ FND-10 |
| FND-01 | The model and template directories are global statics set at startup. Move them into the DI composition root. FND-06 resolves every folder once in `AppPaths` (`paths.rs`) and injects them; the `MODELS_DIR` and template-dir statics remain because the global STT engines and the template loader still read them. Remove them when those modules take the paths as constructor arguments. Templates: ✅ NOT-04 deleted the JSON loader, both template-dir statics and the bundled `templates/` resources; `AppPaths::legacy_templates` is read once to import old custom files. | CAP-01 (STT engines) / ✅ NOT-04 (templates) |
| FND-13 | ~~`cargo deny check advisories` fails on inherited dependencies.~~ FND-03 upgraded `anyhow`, `crossbeam-epoch`, `rustls`, `quick-xml` (via `plist`), `der`, `spin`, dropped `atty` (nnnoiseless without `bin`), and documented the rest in `deny.toml`. Each ignore names its card: `h2` 0.3 and `rustls-pemfile` → FND-12 (reqwest 0.11 goes); `ringbuf` 0.4 → FND-05 / CAP-02 (0.5 API); `paste`, `proc-macro-error`, `unic-*` → upstream specta / tauri. | ✅ FND-03 |
| FND-13 | The app crate is still edition 2021 (the new crates are 2024). Migrate it with `cargo fix --edition` once enough legacy code is ported. | Any later card, as its own PR |
| FND-13 | `rustfmt` cannot see modules declared through `legacy_modules!`, so legacy files are not format-checked; each card formats the files it ports out of the list. | Every card that ports legacy code |
| FND-13 | `api_delete_meeting` and the meeting page's metadata and transcript commands are still legacy; port them into the `meetings` plugin. | ORG-07 / TRN-01 |
| FND-13 | ~~`_example` has no dev route.~~ The dev-only `/design` page renders it below the token and primitive galleries. | ✅ FND-10 |
| FND-13 | ~~Wire `createRootRouteWithContext({ queryClient })`.~~ Done; `/meetings` and `/meetings/$meetingId` warm their queries in loaders. | ✅ FND-10 |
| FND-04 | ~~Seed the core schema per FND-07 table group and promote the shared fixture vocabulary into `deino-domain`.~~ Done by FND-07: `CalendarRows`, `MeetingRows`, `PeopleRows`, `TranscriptRows` with stable `fixture_uuid` IDs, read back through the repositories in `deino-core/src/fixture_tests.rs`; `Channel`, `Millis`, `Email`, `AttendeeRole`, `Rsvp` now come from `deino-domain`. The mock serves the same segment IDs (`fixture-ids.ts`); the new tables' rows join `MockDatabase` with the first command that reads them (next row). | ✅ FND-07 |
| FND-07 | When a card adds the first command over a core-schema table (people, speakers, calendar events, folders, tags, action items), it adds that table's fixture rows to `frontend/src/dev/mock-ipc` (`MockDatabase` + a `<feature>.ts` mock), with IDs from `fixtureUuid` so they match the Rust seeder. | ORG-01 / ORG-04 / ORG-05 / ORG-06 / AI-05 / TRN-01 |
| FND-07 | Persist live through the new ports: create the meeting with `MeetingLifecycleRepo::create` (`recording`, `source = live`), append finalized segments with `SegmentWriter::append` (one transaction per batch), stop with `MeetingLifecycleService::transition(StopRecording)` plus `set_timing` (the only writer of `duration_s`) and `set_audio`, and recover with `list_by_status(Recording)`. Then delete `api_save_transcript`'s write through the `transcripts` view. | FND-05b |
| FND-07 | Drop the `transcripts` compatibility view and its two triggers (ADR 0012) in a new migration once nothing reads or writes it: `database/repositories/{transcript,meeting}.rs` (search, paging, delete), `audio/import.rs`, `audio/retranscription.rs`, `api_save_transcript`. The card that ports the last caller drops it. | FND-05b / CAP-08 / CAP-11 / TRN-01 |
| FND-07 | Legacy imports leave `meetings.source = 'live'` (the default; FND-07 could not tell imported meetings apart). Create imported meetings with `MeetingSource::Import`. | CAP-11 |
| FND-07 | The `transcripts` view hides soft-deleted segments, so a legacy `DELETE FROM transcripts` (re-transcription) leaves them behind. Port re-transcription to `SegmentWriter::replace` before anything soft-deletes. | TRN-02 / CAP-08 |
| FND-07 | Typed payloads FND-07 left as nullable, unread JSON: `folders.auto_rules_json` (ORG-05) and `action_items.external_refs_json` (INT-03). Each gets a Rust struct and its repository field. | ORG-05 / INT-03 |
| FND-07 | ~~`meetings.template_id` names a built-in template as text; make it a foreign key to `templates`.~~ NOT-04's migration makes it a reference (`ON DELETE SET NULL`) and maps the six legacy names to their successors. | ✅ NOT-04 |
| FND-07 | People list aggregates (`people.last_met_at`, `meeting_count`, §3A) were not created: add them with their single writer when the people list sorts by last met; merging people must move speakers, attendees and action items to `merged_into`. | ORG-04 |
| FND-07 | `calendar_events` has no all-day flag and no per-calendar (sub-account) table; add them with calendar selection in Settings. | ORG-01 |
| FND-04 | ~~`MockAudioSource` playing a fixture's tracks.~~ Done in `deino-audio` by FND-05a: `MockAudio::from_env()` parses `DEINO_MOCK_AUDIO` (`1` = generated voices, otherwise a fixture ID, debug builds only), `TestSignal::from_pcm` resamples a `PcmTrack` to 48 kHz and `MockAudioSource::playing` plays both tracks (`tests/finalize_round_trip.rs` round-trips the standup fixture). What is left is the wiring in `composition.rs`. | FND-05b |
| FND-05a | Wire `deino_audio::Recorder` into the live recording: start it with `CmafEncoderFactory`, `SystemHostClock`, `SystemDiskSpace`; feed each capture source's 48 kHz host-stamped blocks into its `RtSink` (or `MockAudio::from_env()` sources in debug builds); call `find_unclosed` + `recover_recording` at launch before moving `recording` meetings to `processing`; store the recording directory under `app_data_dir`. Remove the 30 s checkpoint saver and IndexedDB transcript recovery once this works. | FND-05b |
| FND-05a | `finalize_audio` stage (order 10): call `deino_audio::finalize_recording(dir)` and store `mic.m4a`, `system.m4a`, `audio.mp4` in `meetings.audio_*_path`. Finalize is idempotent and writes through temporary names. FND-08 shipped the runner: the stage is a `PipelineStage` in `crates/deino-pipeline/src/stages/` plus one line in `registry::stages`; it needs the recording directory, which FND-05b stores (FND-08 left it out for that reason). `finalize_recording` blocks on AVFoundation, so run it with `spawn_blocking`. | FND-05b |
| FND-05a | Real `AudioSource` adapters: the cpal microphone and the Core Audio process tap push 48 kHz mono blocks stamped with `AudioTimeStamp::mHostTime` (same domain as `SystemHostClock`) through `RtSink::push` only; resample from the device rate on the conditioner side (§3C). | CAP-02 |
| FND-05a | Show `deino_audio::HealthEvent`s (`Stalled`, `FramesDropped`, `DiskLow`, `StorageFailed`) in the record pill; add the 256 MiB "disk exhausted → stop cleanly" rule (`audio-capture.md` §5), which FND-05a does not enforce. | CAP-10 |
| FND-05a | Anything that decodes Deino's own AAC files with symphonia 0.5 (re-transcription, imports) must trim the 2112 priming frames (`deino_audio::fmp4::AAC_PRIMING`) or decode with ExtAudioFile/AVFoundation. | CAP-08 / CAP-11 |
| FND-05a | CI: `cargo check -p deino-audio` on Linux and Windows (only macOS was built locally; non-macOS paths are `cfg`-stubbed), and run `cargo test -p deino-audio --test kill_harness -- --ignored` on the macOS runner. Re-measure CPU per track in a release build at load < 20 (measured 2.7 % per track in a debug build at load ~99, mock sources included). | FND-03 |
| FND-05a | Not covered: real power loss, WKWebView playback of the fragmented `.m4a.part` files (finalize writes flat files), two recordings at once. cidre a9587fa lacks `finishWritingWithCompletionHandler:`, `AVAssetExportSession` and `ExtAudioFileSetProperty`, so `deino-audio` uses objc2 and a three-function FFI; drop them when cidre gains the bindings. | FND-05b / any cidre upgrade |
| FND-04 | ~~Seed `userNotes` into `meeting_notes` once the notepad reads it, and serve it from the notes mock.~~ `NoteRows` seeds them (converted with `deino-doc`); the `notes` mock serves them from `MockNotes`. | ✅ NOT-01 |
| FND-04 | The meeting page's legacy commands (`api_get_meeting_metadata`, `api_get_meeting_transcripts`, `api_get_summary`) are answered by the dev:mock legacy shim (`src/dev/mock-ipc/legacy-meetings.ts`); move them into the typed `meetings` mock when the commands are ported, and delete the shim entries. | ORG-07 / TRN-01 |
| FND-04 | ~~Run `pnpm test:e2e` and upload `frontend/test-results/` screenshots in CI.~~ FND-03 runs it as the `e2e` step of `scripts/ci-local.sh` and in the Linux `frontend` job (ubuntu-24.04 has Google Chrome), and uploads the screenshots as an artifact. | ✅ FND-03 |
| FND-04 | ~~The sidebar starts collapsed.~~ The sidebar starts open and meetings live on the Meetings page; the smoke tests now go through it. | ✅ FND-10 |
| FND-04 | dev:mock always reports onboarding as completed; add a way (a search param or a dev toggle) to exercise the onboarding flow on mock data. | FND-11 |
| FND-04 | AI evals: score notes, action items, insights and chat against each fixture's `groundTruth` (decisions, action items with owner and due date, facts with evidence quotes). | NOT-02 / AI-05 / AI-06 / AI-02 |
| FND-06 | ~~The `CredentialSource` port reads keys through `SecretStore::get`; delete `summary/credentials.rs` and the model-list commands' stored-key fallbacks.~~ Done: `deino_core::settings::LlmAccess` implements the gateway's credential, endpoint and route ports over the Keychain and `Settings::ai`; `summary/credentials.rs` and the five provider modules are gone. What is left is `api_test_custom_openai_connection` (`api/api.rs`, its own `reqwest` call through `features::secrets::typed_or_stored`): replace it with `plugin:llm\|models` for `custom_openai` with the draft endpoint and key, then delete it and `typed_or_stored`. | FND-09 ✅ / next card touching Summary models |
| FND-06 | The STT code still reads the engine through the string-typed bridge `api::current_transcript_config` (`"parakeetUnified"`/`"parakeet"`/`"localWhisper"`; CAP-01 added `TranscriptionEngine::ParakeetUnified` as the default and kept the bridge, since the legacy engine code it feeds is what CAP-02/FND-05b replace); read `Settings::transcription` (`TranscriptionEngine`) directly. | CAP-02 / FND-05b |
| FND-06 | ~~Wire the theme switch (`Settings::ui.theme`) to the app shell.~~ Settings → Appearance and the ⌘K palette save `ui.theme`; `SettingsSync` applies the saved theme, with `localStorage` (`deino.theme`) only as the first-paint cache. The settings screens are tokenised and sectioned; `ModelSettingsModal.tsx` (1.3k lines), `ConfigContext` and the inherited settings panels are still legacy markup: rebuild them as `features/settings/` sections and rows as their cards port them. | ✅ FND-10 (panels: next settings card) |
| FND-06 | The onboarding commands (`onboarding.rs`) keep their legacy snake_case shape over `Settings::onboarding`; replace them with a typed onboarding plugin. | FND-11 |
| FND-06 | `Settings::privacy` and `Settings::profile` are empty placeholder sections. `profile`: ✅ NOT-05 (name, emails, role, company, focus areas, writing style, context). | PRV-01 / ✅ NOT-05 |
| FND-06 | Transient UI state still uses localStorage (`downloading-models`, `transcription_progress`); move it into Rust-owned state when the model manager and import cards are ported. | CAP-01 / CAP-11 |
| FND-06 | A crash between the first launch's legacy import and the first clean shutdown leaves the dropped keys' zeroed pages in the WAL until the next checkpoint (the import itself checkpoints right after it runs). Consider a `wal_checkpoint(TRUNCATE)` on every open. | FND-05 / FND-07 |
| FND-10 | The sidebar width and collapsed state and the meeting page's right-panel state are not persisted (`features/shell/layout-store.ts`); add them to `Settings::ui` and save them like `ui.theme`. The inherited `ui.transcriptPaneRatio` is unused since the meeting page moved to tabs; drop it or reuse it for the right panel's width. | Next card touching the shell |
| FND-10 | The sidebar's live record row cannot stop a recording: Stop still lives on the record screen, which owns the stop-and-save flow. Add the hover Stop button (ui.md §3 Sidebar) once Rust owns the recording lifecycle. | FND-05 / CAP-10 |
| FND-10 | Set each Tauri window's native background to `--bg` for its theme and show windows after the first paint (no white flash in dark mode), and broadcast theme changes to the `record_panel` window (ui.md §2.9). | CAP-10 |
| FND-10 | `legacy-files.mjs` lists `src/components/**` as one glob, so a ported file (for example `EmptyStateSummary.tsx`, rewritten on the primitives) cannot leave the list on its own. Expand the glob into explicit files. | FND-03 / next card porting a component |
| FND-10 | Home is still the inherited record screen. Build Home (Coming up, Recent notes, empty state with shortcuts) per ui.md §4.1. | ORG-02 |
| FND-10 | The meeting header shows the date chip and resting "Add attendees" / "Add tag" chips; fill them, add the Share/Export menu and the Enhance button. | ORG-04 / ORG-06 / EXP-01 / NOT-02 |
| FND-10 | The Notes and Transcript tabs render the inherited panels through `features/meeting/legacy/` (`panels-context.tsx`). Replace each slot in `features/meeting/slots.ts` and delete the bridge: the Enhanced / My notes `SegmentedControl` (NOT-01/NOT-02) and transcript bubbles with speaker colours (TRN-01). NOT-01 added the Summary / My notes switch (`slots/notes-slot.tsx`, ⌘E) and "My notes"; the Summary view and the Transcript tab still use the bridge. | NOT-02 / TRN-01 |
| FND-10 | The inherited settings panels (General, Recording, Transcription, Summary models, Beta) are tokenised but keep their own card markup. Rebuild them on settings sections and rows (ui.md §3 Settings layout) as their values move into the settings store. | FND-06 |
| FND-10 | dev:mock does not answer `api_delete_meeting`, so deleting a meeting fails there (the real app works). Answer it in the legacy shim, or in the typed mock once ported. | ORG-07 |
| FND-09 | ~~The gateway's HTTP goes through the temporary `ReqwestTransport`.~~ FND-12 gave every provider a `deino-net` scoped client (`llm/http.rs` `NetTransport`). The eval harness keeps its own `reqwest` dev-dependency (`crates/deino-llm/tests/llm_eval/live.rs`); move it onto `deino_net::ReqwestNetClient` with a shared transport when convenient. | ✅ FND-12 (eval harness: next card touching `deino-llm`) |
| FND-09 | Usage page: list and sum `llm_calls` (per route, provider and model over time; tokens, latency, failures). The table and its indexes exist; add the repository queries and the screen. | Next AI card (AI-02) or a small usage card |
| FND-09 | The legacy summary's `reasoning_stripped` flag now reflects only `<think>` envelopes removed from text: providers drop structured reasoning (`reasoning_content`, Anthropic thinking blocks) before it reaches the app, so the "reasoning was filtered" toast no longer appears for those. | NOT-02 (drop the flag with the legacy summaries) |
| FND-09 | Ollama: the legacy `ollama list` CLI fallback (when the HTTP API failed) and the per-pull duplicate guard in Rust were not ported; pulls and deletes use the saved endpoint (the form's unsaved endpoint only applies to model lists). | Next card touching Summary models / FND-11 |
| FND-09 | The local embedding model (`nomic-embed-text-v1.5`, `<app_data_dir>/models/embedding/<name>.gguf`) has no download yet: `Gateway::embed` and the `embed` route test fail with `unavailable` until AI-01 adds it to the model manager. llama-helper's `embed` request was verified only by unit tests (no embedding GGUF on the dev machine). | AI-01 |
| FND-09 | llama.cpp's own load logs (thousands of `create_tensor` lines per model load) still reach the app's stderr; route them through `llama_cpp_2::send_logs_to_tracing` or filter them. The helper's own diagnostics carry only counts and timings. | Next card touching llama-helper |
| FND-09 | The real-app click-through of Settings → AI was done in `pnpm dev:mock` only: the dev binary run with the throwaway identifier started, migrated (`llm_calls`) and wired the gateway, but the computer-use tools cannot drive an unbundled dev binary. The gateway, sidecar adapter and llama-helper were exercised end to end by `llm::smoke_tests` (streaming, grammar-structured output, a full legacy summary with Gemma 3 4B). | FND-03 (a signed debug bundle for UI smoke runs) |
| FND-09 | `CLAUDE.md`'s crate table does not list `deino-llm` yet; add the row (gateway, routes, providers, prompts, `MockProvider`). | Orchestrator |
| FND-09 | Settings → AI lists providers with masked keys but keys are still entered in the inherited Summary models dialog (`ModelSettingsModal.tsx`); move key entry and the Ollama/custom endpoints onto the AI page and retire the dialog's provider pickers in favour of the route table. | NOT-04 / next settings card |
| FND-13 | The real-app UI click-through of the sidebar rename was not completed (the background UI session was interrupted); the IPC path is covered by contract tests and the app was verified to start and migrate an existing database. FND-03 (CI only) did not take it. | Next card touching the sidebar (FND-10) |
| FND-08 | Recording does not reach `processing` through `MeetingLifecycleService` yet, so the pipeline only runs from `rerun` (the strip's retry, or a meeting left `processing`). When the recorder stops with `transition(StopRecording)`, the pipeline's `legacy_summary` stage summarises the meeting: remove the meeting page's auto-summary on stop (`useSummaryGeneration` with `source = 'recording'`) in the same PR so it does not run twice, and have the stage honour `Settings::ai.auto_summary` (skip with a reason when off; a manual re-run still runs). | FND-05b |
| FND-08 | `legacy_summary` reports every failed legacy summary job as retryable: `SummaryJob` records only a message on `summary_processes` (which may quote the provider, so it stays in the summary panel). Return the `deino_core::summaries::SummaryError` (with `LlmError::retry_after`) from the job so the stage can tell a missing key or model (permanent) from a rate limit (retryable). | NOT-02 (or the next card touching summaries) |
| FND-08 | Replace `legacy_summary` (order 50) with `enhance_notes`: delete the stage, the `Summarizer` port and the app adapter; startup prunes the old jobs. The legacy summary panel does not notice a summary the pipeline started until it is reopened; the new notes view should read the stage's status. | NOT-02 |
| FND-08 | `PipelineService::cancel(meeting)` has no command. Cancel a meeting's pipeline when it is trashed or deleted, and offer Cancel next to a running stage if a stage turns out to be long (transcription re-pass). | ORG-07 / CAP-08 |
| FND-08 | The processing strip sits under the meeting header's chips as a plain row of stages. Give it its final place and look (a quiet header line or the Details panel; ui.md has no spec for it yet), with a loading skeleton, and add a Playwright check of the retry flow (dev:mock serves the one-on-one with a failed summary). | FND-10 follow-up / next card touching the meeting page |
| FND-08 | Real-app smoke ran the dev binary under a throwaway identifier against a seeded database: startup pruned a retired stage, recovered a crash-left `running` job (started "in the future", the clock-skew case), skipped meetings without transcript, failed the one with no summary model, and moved each meeting to `ready`/`failed`. Not done in the real app: clicking the strip's Retry (verified in `pnpm dev:mock`; covered by contract tests) and a summary generated end to end by the stage with a configured model. | FND-03 (signed debug bundle for UI smoke runs) |
| FND-08 | Each new stage adds its readable name to `frontend/src/features/pipeline/copy.ts` (`stageLabels`) and its fixture state to `src/dev/mock-ipc/pipeline.ts`; otherwise the strip shows the raw stage ID. | Every card that adds a stage |
| FND-03 | Coverage is not reported in CI yet (`cargo llvm-cov` for the portable crates is cheap on Linux; Vitest coverage needs `@vitest/coverage-v8`). | The next card that touches CI |
| FND-03 | Branch protection that requires `rust (portable crates)`, `rust (app on Linux)`, `frontend` and `pr title` needs GitHub Pro (or a public repo) for a private personal repository. Until then, `scripts/ci-local.sh` before every merge is the gate. | Owner |
| FND-03 | Local `pnpm run tauri:build` in a checkout under `~/Documents` can still fail at `codesign` (provenance and File Provider attributes). Reuse `release-macos.yml`'s `beforeBundleCommand` override in a `tauri:build:dmg` script, or build outside `~/Documents`. | Next card that ships a local build script (or FND-05) |
| FND-03 | `.github/verify-portable-ggml.cjs` and its test are no longer run by any workflow (the Windows distribution workflows were upstream's); `docs/BUILDING.md` documents running them by hand. | Delete with the Windows build story, or when a Windows job returns |
| FND-03 | `scripts/ci-local.sh` runs its steps one after another. The frontend steps (about 80 s) could run beside the Rust compile once the machine is not shared by many agents. | Any later CI change |
| CAP-01 | Persist the live lane's segments with their timed words (`FinalSegment.words` → `transcript_segments.words_json`, `deino_stt::Lane` → `Channel`, `deino_stt::Millis` (u64) → `deino_domain::Millis` (u32)) from Rust, instead of the legacy `transcript-update` event and the frontend save. | FND-05b |
| CAP-01 | Split the live lane into microphone and system lanes: `EngineSlot::Live(Lane::Microphone/System)` already run in separate workers; feed each from its own capture track and label lines Me/Them. | CAP-02 |
| CAP-01 | On a CPU fallback the live stream cannot keep up (spike: RTF 1.05 at the default window). Switch that lane to batch-after-utterance (VAD-bounded runs) or the 1040/1040 window, and raise a recording-health warning (the runtime logs `live transcription runs on the CPU`). | CAP-10 |
| CAP-01 | Offer "Responsive" (5600/560/320, default) and "Accurate / low power" (5600/1040/1040) live windows in typed settings; read the window menu from the GGUF (`stt.parakeet.encoder.att_chunk_*_choices`) instead of probing a stream. | FND-06 |
| CAP-01 | ~~Replace `stt/http_source.rs` with `deino-net`.~~ FND-12: `NetByteSource` over `net.approved_download(HfModels)`; Whisper, Parakeet TDT and the built-in model downloads too. | ✅ FND-12 |
| CAP-01 | Onboarding now downloads Parakeet Unified (recommended quant) through the `transcription` plugin; its "Transcription Engine" step still says "Parakeet" and shows no download speed. Rewrite it with the typed API and show the NVIDIA notice. | FND-11 |
| CAP-01 | `build.rs` and `docs/GPU_ACCELERATION.md` still describe whisper.cpp/CoreML features; `coreml` and `openblas` are now no-op features kept for scripts; `.github/*-portable-ggml*` and `scripts/dev-env.sh` still mention whisper.cpp. Clean them up. | The next card that touches CI or the build |
| CAP-01 | The Parakeet TDT (ONNX) and Whisper engines remain under "Advanced" with their legacy managers; Parakeet TDT v3 also exists as a transcribe-cpp GGUF (`handy-computer/parakeet-tdt-0.6b-v3-gguf`), which would let `ort` go once diarization no longer needs it. | CAP-03 / later cleanup |
| CAP-01 | Batch fallback restarts a lost stream at the loss point, so a word spoken across that instant can be split; cut at the next decoded silence instead. | CAP-02 |
| CAP-01 | A legacy install whose old database had no transcription row imports as the new default (Parakeet Unified), which needs a model download before the first recording; recording start reports the missing model. Consider keeping such installs on Parakeet TDT when its model is already on disk. | FND-11 |
| CAP-01 | Per-process CPU accounting for the app and its STT workers (`EnginePool::worker_pids`), needed for CAP-02's "≤ 1.6× single lane" and CAP-08's 15% acceptance. | CAP-02 / CAP-08 |
| FND-12 | Delete the legacy `api_test_custom_openai_connection` (it tests an unsaved endpoint with its own `reqwest` client, outside the egress log) in favour of Settings → AI's route test after saving the endpoint; then drop `reqwest` from the app's `[dependencies]` and `clippy::disallowed_types` from `legacy_lints.rs`. | NOT-04 / next settings card |
| FND-12 | Offline mode, or a switched-off provider, reaches the gateway as `HttpError::Connect` with a user-safe reason; route notes to the built-in model instead of failing (check `network.offlineMode` in the `Router`, or add a typed `HttpError::Blocked`). | Next card touching `deino-llm` |
| FND-12 | The built-in model downloads (`summary_engine/model_manager.rs`) go through `deino-net` but have no pinned size or SHA-256 and keep their own resume loop; pin both and switch to `deino_net::HttpDownloader`. The Parakeet TDT and Whisper downloads likewise keep their legacy loops. | Next card touching the model managers |
| FND-12 | Connecting an integration switches its `ServiceId` on through `ServiceSwitch::set_enabled` (and disconnecting switches it off); providers receive `net.scoped(..)` from the composition root. | INT-00 |
| FND-12 | Webhook URLs must add their hosts in `SettingsServicePolicy::custom_hosts` (`composition/network.rs`), like the custom LLM endpoint. | DEV-01 |
| FND-12 | The egress log keeps the newest 10,000 rows; add it to retention settings and a "Clear log" action. | PRV-01 |
| FND-12 | The Network page's `SettingsSection`/`SettingsCard` (`features/network/components/settings-section.tsx`) are the ui.md settings section and card; promote them to `ui/` when the inherited settings pages are rebuilt on them. | FND-10 follow-up (settings panels) |
| FND-12 | Acceptance "a full meeting with no services connected makes zero outbound connections" was checked on a fresh smoke build (`com.xenshreyas.deino.smoke-fnd12`) from launch to idle only: `lsof -i` and `nettop` showed no sockets. Repeat it for record → notes with the local model → export once FND-05b lands. | FND-05b |
| CAP-03 | The `diarize` stage downloads its 32 MB of models on the first meeting it processes (retried, then skipped so notes still run); offer the download in the model manager and onboarding instead, so the first meeting is not the trigger. | FND-11 / model manager UI |
| CAP-03 | Attribute WeSpeaker ResNet34-LM (CC-BY-4.0, text in `THIRD_PARTY_NOTICES.md`) on the About screen. | FND-10 / the About screen's next card |
| CAP-03 | Drive `deino_diarize::online::LiveSpeakerTracker` from the system lane: embed finalized utterances ≥ 1.5 s off the lane thread with `Diarizer::embedder()` (or an `EmbeddingExtractor`), label live speakers, and let the post-meeting pass replace them (map live renames by word overlap). | CAP-02 |
| CAP-03 | Use the stored centroids (`SpeakerRepo::voice`), `deino_diarize::voiceprint` (`pick_unique`, `complete_by_elimination`, `select_speaker_spans`) and `KnownSpeaker`s in `DiarizeRequest` (add them to `TrackRequest`) for `identify_speakers`; link "Me" to the `people.is_me` row. | CAP-04 |
| CAP-03 | Re-tune the 0.7045 threshold and re-time on real meetings: run `diarize-bench file` on AMI dev / VoxConverse RTTMs (not downloaded here) and on an idle M1; every DER so far uses synthetic voices (`docs/perf/diarization.md`). | CAP-04 / a tuning pass |
| CAP-03 | Decoding a 60-minute track holds about 230 MB of 16 kHz samples while it runs; add a file-backed `AudioSource` (`ExtAudioFileSeek`) if memory matters. | Later cleanup |
| NOT-04 | ~~Register the `/template` command in the TipTap notepad: `Extension.create({ addProseMirrorPlugins() { return [Suggestion({ editor: this.editor, ...templateSlashSuggestion(deps) })] } })` with `deps = { templates: () => cached list, setMeetingTemplate }` (`features/templates`, `slash/template-slash.ts`) and render `TemplateSlashList` in the suggestion popover. The module is TipTap-free (structural types), tested with a fake editor.~~ Registered in `features/notepad/editor/extensions.ts` (`TemplateCommand`). | ✅ NOT-01 |
| NOT-04 | Generate notes with `MeetingTemplateService::auto_choice` (rules → default → `tagging` classifier → General), record the template used in `enhanced_notes.template_id`, and consider persisting an automatic choice so the classifier runs once per meeting. Retire the legacy summary toolbar's own template dropdown (it now writes the meeting's template through `useLegacyTemplateChoice`). | NOT-02 |
| NOT-04 | Export copies a template's JSON to the clipboard (WKWebView has no reliable `<a download>`); add a native save dialog when a dialog plugin is on the JS side. | Next card touching file dialogs |
| NOT-05 | When the profile's first address belongs to an existing person, that person becomes "Me" and the previous "Me" row stays as a plain person; merge the two (attendance, speakers, action items) with ORG-04's merge. | ORG-04 |
| NOT-05 | `profile::InternalDomains` is the shared internal-vs-external rule: fill `calendar_events.is_external` with it, and use it for follow-up recipients and briefs. | ORG-01 / NOT-07 / NOT-08 |
| NOT-04 | The two follow-ups FND-09 and FND-12 assigned to "NOT-04 / next settings card" (move key entry onto Settings → AI; delete `api_test_custom_openai_connection`) are settings work outside the templates card and remain open. | Next settings card |
| NOT-01 | The record screen's notepad writes into a `scheduled` draft meeting (created on the first edit) and `adoptLiveNotes` moves it into the meeting the legacy stop path saves (`notes\|adopt`, one call in `hooks/useRecordingStop.ts`). Once Rust creates the meeting at recording start, point `LiveNotepad` at it and delete `live-notes.ts`, `NoteService::adopt` and `NotesRepo::transfer`. | FND-05b |
| NOT-01 | Opening an upcoming event's note: call `notes\|create_scheduled` with the event's title and link it with `MeetingLifecycleRepo::link`; ⌘N "New note" still opens the record screen, whose notepad makes a draft. | ORG-01 / ORG-02 |
| NOT-01 | The record panel's "Open notes" should open `NotepadWindow` (exported by `features/notepad`) in its own window on the live meeting; `plugin:notes:notes-saved` lets the two notepads tell their saves apart. | CAP-10 |
| NOT-01 | TipTap's `code` mark excludes every other mark, `origin` included, while `deino-doc` lets code carry provenance; extend the Code extension's `excludes` when AI text first carries inline code. Replace `SummaryDocument` with the enhanced notes. | NOT-02 |
| NOT-01 | Deleting a meeting (legacy `api_delete_meeting`) leaves its `<app data>/meetings/<id>/images/` folder; delete it with the meeting (and keep it while in the trash). | ORG-07 |
| NOT-01 | Index `meeting_notes.notes_markdown` (Readable Markdown, mentions as `@Name`) in `search_fts`. | ORG-08 |
| NOT-01 | Dragging an image over the window still shows the audio-import overlay (only the drop is classified); hide it when every dragged path is an image. ⌘⇧V reads `navigator.clipboard`, which WKWebView may refuse: switch to the clipboard plugin if Manual QA shows it. | Next card touching the shell |
| NOT-01 | React StrictMode stays off (`main.tsx`): TipTap is safe, but the legacy contexts' mount effects are not idempotent. | Whoever ports the last legacy context |
| CAP-06 | Recording starts through the shell's start path (`ShellRecordingStarter` emits `RecordingRequested`; the record screen starts the legacy recorder) and "recording" is the legacy `IS_RECORDING` flag (`LegacyRecordingStatus`). Switch both to the typed `recording.start` / recording state: start with the detection's app and linked event as the meeting title, and open that meeting's notepad (NOT-01) instead of the record screen. | CAP-02 (then NOT-01) |
| CAP-06 | Calendar association ships as `NoCalendar`. Implement `CalendarLookup` over `calendar_events` with `deino_detect::calendar::pick_linked` (started < 15 min ago, starts within 5 min; skip all-day and declined events), and let the prompt's event line open the event. | ORG-01 |
| CAP-06 | Browser meetings are recognised by URL only when Accessibility is granted (`reads_browser_urls`); otherwise a browser call prompts after the 15 s hold as "Meeting detected". Request Accessibility with an explainer and a deep link to System Settings, and add the ScriptingBridge active-tab fallback (Muesli) if AX misses Chrome tabs. | CAP-05 |
| CAP-06 | Auto-stop: build on `DetectionEvent::MeetingEnded` (2 s grace today). The policy marks a meeting "handled" while recording and does not track it further; CAP-07 needs the recorded meeting's app and its release (20 s) while recording. | CAP-07 |
| CAP-06 | Detection ignores Do Not Disturb (`notifications.respectDoNotDisturb`) and has no "always detect" include list (anarlog); add both if users ask. The prompt window re-reads the theme cache on each detection only; broadcast theme changes to it with the record panel's. | CAP-10 / ORG-02 |
| CAP-06 | Real-call smoke not observed by the agent: the smoke build (`com.xenshreyas.deino.smoke-cap06`) started detection with all three listeners and read 43 to 46 Core Audio processes, but `ffmpeg` from the agent's shell could not hold the microphone (macOS permission prompt for the host app) and computer-use access to Photo Booth was declined. Run the Manual QA list in `crates/deino-detect/README.md` (Zoom, Meet in Chrome, Slack huddle, ignore list, auto-start). | Owner |
| CAP-06 | `src/dev/mock-ipc/mock-database.ts` imports `@/features/meetings`, whose generated mock imports `database.ts` back; an entry that does not load the meetings feature first hits a TDZ error in `pnpm dev:mock` (worked around in `src/detection-prompt.tsx`). Move `asMeetingId` out of the cycle. | FND-04 owner / next card touching mock IPC |

**Critical path to "Granola-equivalent daily driver":**
1. FND-01 → FND-02 → FND-05 / FND-07 → FND-08 → CAP-01 / CAP-02 → NOT-01 → NOT-04 → NOT-02 → ORG-01 → ORG-02 → AI-01 → AI-02

**Critical path to "Circleback-equivalent follow-through":**
1. everything above, then AI-05 → AUT-01 → INT-01 / INT-03 → CAP-03 → CAP-04 → TRN-03 → AI-06 → DEV-02

**Files that are hot spots** (cards that touch these must not run at the same time; §7 already schedules around them):
- `lib.rs` (mitigated by FND-02)
- `audio/pipeline.rs` (CAP-02 only, then CAP-03 reads only)
- `meeting-details` page (handed to FND-10 slots)
- the Sidebar (handed to the FND-10 registry)
- `summary/*` (frozen after FND-09; NOT-02 replaces it)

---

## 8. Decisions and defaults

Agents use these defaults. The owner can change any of them.

| ID | Decision | Default |
|---|---|---|
| D1 | Platforms | macOS (Apple Silicon) first-class. Windows and Linux compile-only for new features. |
| D2 | Cloud features | **None.** There is no Deino server, meeting bot or sync. The only outbound traffic is directly to the user's LLM provider and connected integrations (§1). |
| D3 | Default AI | Transcription and embeddings are **always local**. Notes, chat and extraction use **Claude or OpenAI** through the user's own API key (Claude defaults in §4.3). The built-in local model is only a fallback when there's no key. |
| D4 | Web search in chat | Off by default. When the chat route is Claude, use Claude's server-side web search tool; otherwise a user-provided Brave or Tavily key. |
| D5 | Bundle id | `com.xenshreyas.deino`. Change it before the first real install, because changing it later moves the data dir. |
| D6 | Auto-update | Disabled. Install DMGs by hand from CI artifacts. Re-enable later with your own minisign key and a release host (the private GitHub repo needs a token, so use a separate public releases repo or R2/S3). |
| D7 | Audio retention | Keep forever by default, since playback is a selling point. Users can configure it under PRV-01. |
| D8 | Telemetry | None. There is no analytics code at all. |
| D11 | FFmpeg (GPL builds) | **Remove the FFmpeg sidecar.**<br>• **Encode** AAC/fragmented MP4 natively with AudioToolbox/AVFoundation (FND-05).<br>• **Decode** imports with `symphonia` (already a dependency), falling back to AVFoundation (`AVAssetReader`) for video containers (CAP-11).<br>• **Cut clips** with AVFoundation (TRN-04).<br>• **Until then:** the sidecar runs as a separate executable (aggregation, not linking), and the last card that uses it deletes `build/ffmpeg.rs`. Never link libav* into the app. |
| D10 | Upstream merges | Keep the `upstream` remote fetch-only. Cherry-pick upstream Meetily fixes by hand when useful. Never merge wholesale after FND-02, because the trees diverge heavily. |

## 9. Setup only the owner can do

Agents should build with config fields for these and document the steps in `docs/setup/<provider>.md`. They should not block on them.
- **Google Cloud OAuth client** (Desktop app type) for Calendar, Gmail and Drive. It stays in "Testing" mode with the owner as test user, so restricted Gmail scopes need no verification.
- **Microsoft Entra app registration** (public client, loopback redirect) for Graph Calendar and Mail.
- **Slack app** (create from the manifest at `docs/setup/slack-manifest.yaml`), **Notion integration**, **Linear OAuth app** or API key, **HubSpot private app**, Attio, Salesforce connected app, Todoist and Asana tokens.
- **Anthropic API key** (and any others) pasted into Settings → AI.
- **macOS permissions** granted manually: Microphone, Screen & System Audio Recording, Accessibility, Calendars, Reminders, Notifications.
- **Apple Developer ID** signing and notarization (optional; ad-hoc signing works for personal use). Required for the CAP-13 watermark camera extension.

## 10. Future considerations (not now)

- **iPhone app** for in-person recording and reading notes. It needs a sync design with **no Deino server**, such as an iCloud/CloudKit private database or direct Mac↔iPhone sync over the local network. Design it only after the Mac app is complete, and record the design as an ADR.
- **Apple Watch recorder**, after the iPhone app.

## 10A. Explicitly out of scope

- Transcription model accuracy and benchmarking, multilingual support and translation (owner constraint).
- Everything in §5.8: meeting bot, share links, sync, mobile and Watch apps, Slack bot, teams and admin.
- Granola "Heads Up" Calendar add-on, Granola phone dialer, Crunched-style social sharing.
- Sales conversation analytics (talk ratios, call scoring). Neither target has it, and Circleback concedes it to Gong.
- Billing, plans, referrals and paywalls (this is a personal tool).

## 11. Sources

- `docs/research/granola.md`: built from about 110 help-center pages, the API docs and the changelog, fetched 2026-10-08.
- `docs/research/circleback.md`: built from the help center (`llms-full.txt`), the OpenAPI spec and 219 release pages, fetched 2026-10-08.
- `docs/research/community.md`: Meetily GitHub issues and PRs, HN, Product Hunt, reviews, and OSS alternatives (Anarlog, Muesli, OpenWhispr, LokalBot, Scriberr).
- `docs/research/meetily_audit.md`: a code-level audit of this repo at the baseline commit.
- `~/Documents/GitHub/Handy`: the reference implementation of the Parakeet Unified EN GGUF engine (`transcribe-cpp`).
