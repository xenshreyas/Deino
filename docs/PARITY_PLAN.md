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
| `year_recaps` (and the `stats_meetings` view) | ORG-09 | Stats are computed on read; only the year in review is kept |
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
| **Chat-bubble layout** (Me right, Them left) and speaker colours | ✅ | ✅ | ✅ (TRN-01, `features/transcript`) | TRN-01 |
| **In-transcript search (⌘F), including by speaker** | ✅ | ✅ | ✅ (TRN-01) | TRN-01 |
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
  - [x] Each stop path is covered by a unit test with a simulated clock.
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
  - [x] Numbers match the fixture data (`crates/deino-core/src/stats/stats_tests.rs`).

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
| ORG-05 Folders | ☑️ done on branch `feat/ORG-05-folders-tags` (ADR 0024; `folder_rules` table, stage `auto_folder` at 75, plugin `folders`, sidebar tree with drag and drop, folder pages, Unfiled) | |
| ORG-06 Tags and AI auto-tagging | ☑️ done on branch `feat/ORG-05-folders-tags` (ADR 0024; stage `auto_tag` at 70 on route `tagging`, plugin `tags`, event `MeetingTagged`, Tags page, header chips, tag filter) | |
| ORG-01 Calendar sync | ☑️ done on branch `feat/ORG-01-calendar` (ADR 0027; `calendars` table, crates `deino-eventkit` (macOS Calendar) and `deino-connect` (OAuth PKCE, Google Calendar and Microsoft Graph connectors), plugin `calendar`, event `CalendarChanged`, Settings → Calendar, Coming up on Home, CAP-06's `CalendarLookup`) | |
| EXP-01 Exports | ☑️ done on branch `feat/EXP-01-exports` (ADR 0025; crate `deino-export`: Markdown, TXT, JSON, SRT, VTT, DOCX, Typst PDF; `export` plugin; Share menu and export dialog) | |
| EXP-02 Auto-export and full account export | ☑️ done on branch `feat/EXP-01-exports` (stage `auto_export` at 99, `Settings::export`, Obsidian mode, account zip with CSVs; Settings → Export) | |
| FND-05b Rust-owned live persistence | ☑️ done on branch `feat/CAP-02-live-pipeline` (ADR 0026; crate `deino-recording`, plugin `recording`, stage `finalize_audio` at 10, launch recovery; the legacy recorder, frontend saver and IndexedDB recovery removed) | |
| CAP-02 Me/Them channel separation | ☑️ done on branch `feat/CAP-02-live-pipeline` (ADR 0026; capture in `deino-audio::capture`, two lanes with WebRTC AEC3 via `sonora` and the text holdback; with FND-05b) | |
| AI-05 Action items system | ☑️ done on branch `feat/AI-05-action-items` (ADR 0028; stage `extract_action_items` at 60 on route `extract.action_items`, plugin `action-items`, event `ActionItemChanged`, `action_item_views` table, `/action-items` page, Notes-tab block, 09:00 reminder, eval `extract.action_items`) | |
| CAP-04 Speaker naming and voice profiles | ☑️ done on branch `feat/CAP-04-voice-profiles` (ADR 0029; `voiceprints` and `speaker_suggestions` tables, stage `identify_speakers` at 35, `Settings::speakers` thresholds, plugin `speakers`, Participants panel with "Is this …?", rename one segment or all, merge, Settings → People → Voice profiles; known voiceprints merge clusters in `diarize`) | |
| NOT-02 Enhanced notes engine | ☑️ done on branch `feat/NOT-02-enhanced-notes` (ADR 0030; `enhanced_notes` versions, `deino-core::enhance` with the provenance guard and map-reduce, stages `enhance_notes` at 50 and `auto_title` at 65, plugin `enhance`, the Enhanced view, Enhance/Regenerate ⌘⇧E with the "Replace your edits?" dialog, regenerate banner, version history; legacy summaries imported as version 1, then `summary_processes`, `transcript_chunks` and the summary code removed; `notes.enhance` eval) | |
| NOT-03 Citations UI | ☑️ done on branch `feat/NOT-02-enhanced-notes` (the magnifier beside cited lines, ⌘⇧S, the excerpts popover with speaker and time, Play disabled until TRN-03, Show in transcript) | |
| INT-00 Integrations framework | ☑️ done on branch `feat/INT-00-integrations` (ADR 0031; scope cut by D12; crate `deino-integrations`: `Integration` + capability traits, the provider registry with a debug-only sandbox, OAuth PKCE over a loopback redirect with per-provider redirect and client-auth rules, refresh, revocation, a device-code helper, token paste; `CredentialStore` in `deino-secrets` (one Keychain item per connection); `integration_connections`, `integration_connection_scopes`, `integration_deliveries`; idempotent deliveries with a GCRA limiter, retries and per-provider failure classifiers; six new `ServiceId`s; plugin `integrations`; Settings → Integrations with planned providers as WIP; `deino-connect`'s calendar connectors moved onto this OAuth layer and its own deleted) | |
| ORG-08 Global search and ⌘K | ☑️ done on branch `feat/ORG-08-search` (ADR 0032; `search_documents` + FTS5 `search_fts` kept by triggers, plugin `search`, ⌘K search and `@` filter providers, `/search` page, moments open at `?t=`; 2,000 meetings answer in 39–62 ms at load ~40) | |
| TRN-01 Transcript view v2 | ☑️ done on branch `feat/TRN-01-transcript-view` (ADR 0033; `deino-core::transcript`, plugin `transcript` (`get`), the recorder's `transcript` command removed; `features/transcript`: bubbles in speaker turns named from CAP-04's participants with its "Who is …?" on each turn's chip, virtual rows per line, ⌘F search with `speaker:` chips and "n of N", live following and "Jump to now", `?t=` and `?segment=` moments, the `TRANSCRIPT_ACTIONS` registry; `VirtualizedTranscriptView`/`TranscriptView`/`TranscriptPanel`/`LiveTranscript` and the legacy slots deleted; dev:mock long meeting on request) | |
| AI-01 Embeddings and semantic index | ☑️ done on branch `feat/ORG-08-search` (ADR 0032; `nomic-embed-text-v1.5` Q8_0 in the model catalog, `embeddings` BLOBs + in-memory cosine index, stage `index` at 80, backfill and Settings → Search index, RRF `search_hybrid`; eval 10/10 in the top 3 with the real model) | |
| ORG-03 Event linking, series, related meetings | ☑️ done on branch `feat/ORG-03-event-linking` (ADR 0034; `deino-core::calendar::linking`: one `nearest_event` rule shared with CAP-06's detection, `EventLinkService` (auto-link on start through the recorder's new `StartObserver`, link, re-link, unlink, also mid-recording), `EventAttendance` (invitees into `meeting_attendees` through `AttendeeDirectory::person_for`), `RelatedService` (series, or title and people; "Previous in series"); `RegenerateReason::EventChanged` from an `EventBasis` in the enhanced notes' basis; `idx_meetings_title`; `calendar` plugin commands `meeting_event`, `event_suggestions`, `link_event`, `unlink_event`, `related_meetings`; `features/event-links`: the header's event chip and picker, the "Previous" chip, the Details panel's event and related meetings, the chip on the record screen) | |
| DEV-02/04 Read-only `deino` CLI and stdio MCP server (re-scoped by D12) | ☑️ done on branch `feat/DEV-02-cli-mcp` (ADR 0035; crate `deino-cli`: `deino meetings`, `meetings read`, `transcripts read`, `search`, `action-items`, `people`, `companies`, `tags`, `folders`, `calendar`, `export`, `status`, all with `--json`, and `deino mcp` with ten read-only tools over `rmcp`; `Database::open_read_only` with `SchemaCheck`; `deino-core::query` over the app's repositories (newest enhanced notes with cited moments as `deino://meeting/<id>?t=ms` links), search through ORG-08/AI-01's `Retriever` (semantic via llama-helper when the model is downloaded, `mode` reported); `deino-core::cli_setup` and plugin `developer`; Settings → Developer: Install CLI and Claude snippets; `cli` sidecar via `scripts/stage-cli.sh`; `docs/setup/claude-code.md`) | |
| CAP-10 Record panel, recording health, menu bar | ☑️ done on branch `feat/CAP-10-record-panel` (ADR 0039; the `record_panel` window: a non-activating `NSPanel` (never key, first click accepted, click-through outside the pill driven at 30 Hz), the pill with a five-bar level meter over both sides, hover controls (pause, stop, native microphone menu, call audio, Open notes), drag with edge snap and remembered position, light and dark from the main window's theme; `deino-recording::monitor` (`HealthMonitor`: no audio 5 s per track, speaking microphone with no transcript 30 s, transcription stopped, disk under 2 GiB, disk under 256 MiB stops) with issues in `RecordingSnapshot::issues`, the pill, the sidebar and notifications; microphone switch and call-audio toggle mid-recording through the capture supervisor; `tray.rs` out of the legacy list: next meeting countdown, "Xm left", today's meetings with Join / Record / Open; settings `recording.showPanel`, `panelPosition`; the `live_fixture` suite paced by sample count with a transcript-watermark holdback) | |
| CAP-07 Auto-stop and meeting boundaries | ☑️ done on branch `feat/CAP-10-record-panel` (ADR 0039; `deino-detect::autostop`: the tick-driven `AutoStopPolicy` (microphone released 20 s after 30 s of ownership, 15 min without speech, sleep by `NSWorkspaceWillSleepNotification` or a 60 s tick gap, the optional calendar end + 10 min with 2 min quiet, a full disk), the 15 s "Ending…" notice with Keep recording in the pill (a toast when the panel is off), the back-to-back offer under the pill (split = stop, then start linked to the next event), `AutoStopService` with its ticker; Resume recording on a finished meeting (`Recorder::resume` adds a manifest segment at the recorded length, `MeetingEvent::ResumeRecording`); settings `recording.autoStop`; every stop path tested on a fake clock) | |
| ORG-04 People and Companies | ☑️ done on branch `feat/ORG-04-people-companies` (ADR 0036; one `entities` service for every source of people and companies, `companies.merged_into`, plugins `people` and `companies`, stage `update_entities` at 90 on route `entity.summary`, merges that move attendance, speakers, voiceprints, suggestions, action items and @mentions in one transaction, service `favicons` (off by default) for company logos, People and Companies lists, person and company pages, hover cards on attendee chips and @mentions, attendee chips in the meeting header; counts and last met computed on read, 39 ms for 2,000 people) | |
| TRN-03 Synced audio and video player | ☑️ done on branch `feat/TRN-02-03-player-edit` (ADR 0040; `deino-core::player` (`PlayerService`, `FsMediaFiles`), plugin `player` (`media`, `download` through Rust's save dialog); the asset protocol scoped to `$APPDATA/recordings/**` (and note images), every `fs:*` grant and the fs plugin removed; `features/player`: the docked `MeetingPlayer` (play/pause, ±15 s, 0.75–2×, a scrubber with speaker bands, Mixed/Me/Them, the screen recording following the audio, Download, Space and ←/→, `?t=`), `PlaybackController`; the transcript lights the playing line and word, follows it while watched, seeks on click; Play from here; the citations' Play enabled; legacy `AudioPlayer`/`useAudioPlayer` were already gone. Seeks land within 6–11 ms of the fixture's known times (±200 ms required; `e2e/player.spec.ts`) | |
| TRN-02 Transcript editing, deletion, search and replace | ☑️ done on branch `feat/TRN-02-03-player-edit` (ADR 0040; migration `trn02_segment_edits`: `original_text`, a trigger that drops the semantic windows over an edited or deleted line, the legacy view's delete also removing soft-deleted rows; `transcript::TranscriptEditService` (edit, delete, restore, find and replace with a preview that is the plan), `segments::SegmentEditor`, `search::ReindexQueue`; plugin `transcript` gains `edit`, `delete`, `restore`, `preview`, `replace`; `features/transcript/edit/`: double-click or ↵ to edit, Delete with Undo, the replace row (⌥⌘F, Match case, this meeting or all, counted confirmation), "Transcript changed: Regenerate notes?" from NOT-02's reasons, the toolbar's ⋯ menu; deleted and replaced text proven absent from the view, the notes engine's input, every text export, keyword and semantic search and the CLI's `query::Library`) | |
| TRN-05 Custom vocabulary and corrections | ☑️ done on branch `feat/TRN-05-vocab-capture-modes` (ADR 0037; `vocab_terms`, `vocab_variants`, `vocab_corrections`; `deino-core::vocabulary`: terms with sounds-like spellings and replacement rules, people, companies and invitees added automatically, whole-word case-insensitive matching with Jaro-Winkler 0.92 for names of several words (0.95 with guards for one word), property-tested; the live pass around the recorder's store and events; stage `vocab_fixups` at 40; undo that never overwrites an edit; the enhanced notes' vocabulary input; plugin `vocabulary`; Settings → Vocabulary; corrections with Undo in the meeting's Details panel. Parakeet in transcribe-cpp 0.3.1 takes no hotwords, so corrections only) | |
| CAP-08 Recording-only mode and post-pass transcription | ☑️ done on branch `feat/TRN-05-vocab-capture-modes` (ADR 0037; Settings → Transcription → "Live transcription" and "High-accuracy re-pass"; record-only opens no engine and no live pipeline: 6.0 % of one core in `tests/record_only_cpu.rs` (debug build, mock capture, real `AVAssetWriter`, load average 24) vs 10.1 % live on the fake engine; stage `final_transcription` at 20 over `deino-recording::offline::OfflineTranscriber` (ExtAudioFile decode, VAD utterances ≤ 30 s, early-stop re-read) with speakers carried by overlap; fixture end to end in `tests/capture_modes.rs`, 8 of 8 lines after stop) | |
| FND-11 Onboarding v2 | ☑️ done on branch `feat/FND-11-onboarding` (typed `Settings::onboarding` progress (`completed`, `step`, per-step `done`/`skipped`) over a pure step machine; `deino-core::permissions` (`PermissionService` over the `PermissionGate` port) with the macOS adapter `src-tauri/src/permissions` and plugin `onboarding`; six steps: welcome, permissions (mic, system audio, notifications, Accessibility, Calendar; live), AI setup (Parakeet Unified and the embedding model download, a Claude or OpenAI key checked then saved, or the built-in model), calendar (macOS Calendar), profile (Continue saves edits), try it (a 20 s test through the real recorder, or the bundled sample meeting: `deino-core::sample`, imported once through the repositories as `meeting-sample`, deletable with its action items); every step skippable, resumed from Settings → Setup; the database is created at startup; the legacy onboarding components, context and commands deleted; Playwright walk-through with light and dark screenshots) | |
| AI-06 Custom AI insights | ☑️ done on branch `feat/AI-06-insights` (ADR 0038; `insight_defs` (typed fields, Auto/N, revision) and `insight_results` (one run per insight and meeting: pending, done or failed, typed answers attributed to a segment); `deino_domain::conditions` (`Condition`, `ConditionGroups`, shared with AUT-01); `deino-core::insights` (12-template gallery, `Gateway::structured_as` with a per-insight schema on route `extract.insights`, Rust validation of every field, `s12` attribution to speaker and time, CSV through `deino_export::csv::insight_table`); stage `insights` at 85; plugin `insights`; Insights page (one sortable, expandable table per insight, CSV export, "Run on past meetings" as pipeline re-runs with progress), the meeting's Insights tab, the Insights export section and `insights.csv` in the account export; eval `extract.insights`) | |
| NOT-09 Sensitive-info redaction | ☑️ done on branch `feat/NOT-09-redaction` (ADR 0043; `deino-core::redaction`: one `Detector` per `SecretKind` (passwords after a cue, provider keys and high-entropy tokens, Luhn-checked card numbers on a network prefix, SSNs, one-time codes in context) over typed and spoken digits, `Redaction` spans with a `Confidence`, the optional `SpanCheck` (`LocalModelCheck` on the built-in model only, may only reject likely spans; regex-only when the `tagging` route is hosted), `TextRedactor` applied by `MaterialSources::with_redactor` (notes input, titles, anything that gathers material) and `ExportPorts::with_redactor` (exports, auto-export, account zip, the `deino` CLI and MCP, patterns only); `privacy.redactSensitive`, off by default, in Settings → Network → Sensitive information; fixture of 11 secrets and 11 near-misses, property tests) | |
| PRV-02 Encryption at rest and app lock | 🚧 first half on branch `feat/PRV-02-encryption-lock` (ADR 0041): SQLCipher 4.5.7 through libsqlite3-sys `bundled-sqlcipher` on Unix, CommonCrypto on macOS (no OpenSSL); `DbKey` (random 256-bit, zeroized, no `Debug`), `FileFormat`, `Database::open_keyed`/`open_read_only_keyed`, `deino_db::convert` (verified copy, atomic swap, backup until verified) and `recover`, `Fingerprint`; `DatabaseKeyStore` in `deino-secrets`; the `deino` CLI reads the Keychain key for an encrypted file; `docs/setup/encrypted-recordings.md`. Tests: a raw open and a wrong key fail; a seeded database round-trips encrypt → decrypt with every table's row count and SHA-256 equal. Left: app wiring, app lock, Settings → Privacy & security (`docs/handoff/PRV-02.md`) | |
| AUT-01 Automations engine | ☑️ done on branch `feat/AUT-01-automations` (ADR 0042; `automations` (definitions: switches, `ConditionGroups`, ordered typed `Action`s), `automation_runs` (the latest run per automation and meeting: armed, pending, done or failed, with its trigger) and `automation_run_steps` (each step's status, typed failure, error and output); AUT-01's conditions added to `deino_domain::conditions` (participants, title is/is not/regex, invitee emails and count, domains all/not all, duration, calendar; 30-case table test); `deino-core::automations`: the `Step` trait and `StepRegistry` (provider steps and the webhook listed as WIP, D12), local steps (generate insight, add tag, add to folder, export file, notification, shell script: JSON on stdin, 60 s, meeting folder, own process group, behind "Allow shell scripts", off by default), `AutomationPayload` `deino.automation` v1 (`docs/setup/shell-scripts.md`), `AutomationEngine` (idempotent per meeting, resumes after done steps, stop on failure), the tag trigger that arms upcoming meetings; stage `automations` at 95; plugin `automations` (13 commands); Automations page (rail, builder with condition groups, step list, folder picker, run log, Test on latest meeting, Duplicate, Delete; Matching meetings with Run on selected), the meeting's ⋯ → Automations menu with run status and Rerun; the Meetings list's bulk "Run automation"; payloads and exported files redacted like every export (NOT-09); Playwright walk-through with light and dark screenshots) | |
| CAP-05 Speaker names from the meeting app UI (Accessibility) | ☑️ done on branch `feat/CAP-05-speaker-tags` (ADR 0045; crate `deino-speaker-tags`: `SpeakerAdapter` per app (Zoom and Teams over a description scanner, Meet in Chrome over `AXDOMClassList`), unit-tested on 19 fixture AX dumps with a capture tool, the 250 ms `Watcher` thread on the recorder's new `Playhead`, `Timeline` spans; `speaker_tag_spans` table; `deino-core::speaker_tags` (`align` > 50 %, `resolve`, `seats`, `SpeakerTagPriors`) run first in `identify_speakers` and override voice matching, never the user; names never create people (unresolved ones label the speaker); `StartObservers` in `deino-recording`; plugin `speaker-tags` (`status`, `open_accessibility_settings`); Settings → Speaker tags with Enabled / Needs permission / Unsupported per app) | |
| Fix: webview open commands and slop tests | ☑️ done on branch `fix/open-url-and-slop-tests` (`system_open`: `open_external_url` opens only `https:`/`http:`/`mailto:` parsed with `url`, `open_meeting_folder` only folders inside the app data or recordings folder, `open_system_settings` only plain pane names; 19 low-value tests from the test audit deleted, the `audio/import.rs` and `features/meetings` ones left to their cards) | |
| ORG-09 Stats and year in review | ☑️ done on branch `feat/ORG-09-stats` (ADR 0049; `deino-core::stats`: `StatsService` computes a period's meetings, hours, people met, internal vs external (`InternalDomains`), top people, local weekdays and hours, the last year's heatmap and LLM usage per provider and model with an estimated cost (`pricing.rs`) on read over the `stats_meetings` view (not trashed, not scheduled, not the sample); `RecapService` writes the year in review on the new `recap` route from numbers and titles only, December or `DEINO_RECAP_ANYTIME=1`, kept in `year_recaps`; plugin `stats`; the Stats page with the heatmap, period control (30 d, 90 d, 12 mo, all time), KPI row, internal/external bar, top people, busiest days and hours, LLM usage; every number asserted over the fixtures in `stats_tests.rs`; 2,000 meetings + 20,000 calls in ~45 ms (release, load ~100) in `stats_bench.rs`) | |
| ORG-07 Meetings list, views, bulk actions, trash, merge | ☑️ done on branch `feat/ORG-07-meetings-list` (ADR 0046; `meeting_views` and `meeting_merge_parts`; `deino-core::meetings` `list` (typed `MeetingFilter`: date, people, companies, tags, folders with subfolders, has action items, internal/external; cursor pages; `matching_ids` for ⌘A), saved views, `trash` (soft delete filtered at read time, restore, delete forever with the files, `TrashPurger` every 6 h for meetings trashed over 30 days), `merge` (one transaction onto the earliest meeting, parts keep their recordings), `MeetingBulkService`, the `MeetingWork` port (trashing cancels the pipeline, a merge re-runs `index`); `ExportService::save_many`; the Meetings page grouped by day with view tabs, the `F` filter menu, Shift-click and ⌘A selection, the bulk bar (tag, add to folder, export, merge, delete with Undo) and the Trash page; the legacy delete removed. Filtering 2,000 meetings: 3.6 ms in release at load ~60) | |
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
| FND-01 | `onnxruntime.rs` and `ffmpeg.rs` duplicate the sha256 helper. Moot once FFmpeg is removed (D11). FND-05a added no FFmpeg; `build/ffmpeg.rs` stays while import and re-transcription still call the sidecar (FND-05b removed the legacy recorder's use). | The last card that removes an FFmpeg caller (CAP-11 / TRN-04) |
| FND-10a | ~~The record screen's permission card still tells macOS users to install BlackHole.~~ It was no longer rendered; FND-11 deleted `PermissionWarning.tsx`, and onboarding's permissions step explains the process tap. | ✅ FND-11 |
| FND-10a | ~~The sidebar has no `shrink-0` and squeezes at narrow widths.~~ The new sidebar is `shrink-0`, resizable (200–320px) and collapsible. | ✅ FND-10 |
| FND-10a | ~~`tauri build` codesign fails in worktrees under `~/Documents` (file attributes).~~ `release-macos.yml` clears extended attributes in a `beforeBundleCommand`, before Tauri signs the bundle. Local `pnpm run tauri:build` under `~/Documents` still needs the same override (see FND-03 below). | ✅ FND-03 |
| FND-10a | ~~Tailwind v4 default palette replaced the v3 hex values.~~ Every palette colour is now a design token; ESLint bans palette classes and hex values. | ✅ FND-10 |
| FND-01 | The model and template directories are global statics set at startup. Move them into the DI composition root. FND-06 resolves every folder once in `AppPaths` (`paths.rs`) and injects them; the `MODELS_DIR` and template-dir statics remain because the global STT engines and the template loader still read them. Remove them when those modules take the paths as constructor arguments. Templates: ✅ NOT-04 deleted the JSON loader, both template-dir statics and the bundled `templates/` resources; `AppPaths::legacy_templates` is read once to import old custom files. | CAP-01 (STT engines) / ✅ NOT-04 (templates) |
| FND-13 | ~~`cargo deny check advisories` fails on inherited dependencies.~~ FND-03 upgraded `anyhow`, `crossbeam-epoch`, `rustls`, `quick-xml` (via `plist`), `der`, `spin`, dropped `atty` (nnnoiseless without `bin`), and documented the rest in `deny.toml`. Each ignore names its card: `h2` 0.3 and `rustls-pemfile` → FND-12 (reqwest 0.11 goes); `ringbuf` 0.4 → FND-05 / CAP-02 (0.5 API); `paste`, `proc-macro-error`, `unic-*` → upstream specta / tauri. | ✅ FND-03 |
| FND-13 | The app crate is still edition 2021 (the new crates are 2024). Migrate it with `cargo fix --edition` once enough legacy code is ported. | Any later card, as its own PR |
| FND-13 | `rustfmt` cannot see modules declared through `legacy_modules!`, so legacy files are not format-checked; each card formats the files it ports out of the list. | Every card that ports legacy code |
| FND-13 | `api_delete_meeting` and the meeting page's metadata and transcript commands are still legacy; port them into the `meetings` plugin. TRN-01 moved the Transcript tab onto `plugin:transcript\|get`; `api_get_meeting_metadata` (and the first transcript page it rides on) remain only for the tab's inherited folder and re-transcribe tools. ORG-07 removed `api_delete_meeting`: every delete is now a trash through `plugin:meetings|trash`. | ✅ ORG-07 (delete) / CAP-08 / CAP-11 / ✅ TRN-01 |
| FND-13 | ~~`_example` has no dev route.~~ The dev-only `/design` page renders it below the token and primitive galleries. | ✅ FND-10 |
| FND-13 | ~~Wire `createRootRouteWithContext({ queryClient })`.~~ Done; `/meetings` and `/meetings/$meetingId` warm their queries in loaders. | ✅ FND-10 |
| FND-04 | ~~Seed the core schema per FND-07 table group and promote the shared fixture vocabulary into `deino-domain`.~~ Done by FND-07: `CalendarRows`, `MeetingRows`, `PeopleRows`, `TranscriptRows` with stable `fixture_uuid` IDs, read back through the repositories in `deino-core/src/fixture_tests.rs`; `Channel`, `Millis`, `Email`, `AttendeeRole`, `Rsvp` now come from `deino-domain`. The mock serves the same segment IDs (`fixture-ids.ts`); the new tables' rows join `MockDatabase` with the first command that reads them (next row). | ✅ FND-07 |
| FND-07 | When a card adds the first command over a core-schema table (people, speakers, calendar events, folders, tags, action items), it adds that table's fixture rows to `frontend/src/dev/mock-ipc` (`MockDatabase` + a `<feature>.ts` mock), with IDs from `fixtureUuid` so they match the Rust seeder. Folders and tags: ✅ ORG-05/ORG-06 (`mock-folders.ts`, `mock-tags.ts`, `organizer.ts`). Calendars and events: ✅ ORG-01 (`calendar-fixtures.ts`, `calendar.ts`, the Work calendar the Rust seeder also writes). Action items: ✅ AI-05 (`mock-action-items.ts` from each fixture's `groundTruth.actionItems`, `action-items.ts`). Speakers: ✅ CAP-04 (`mock-speakers.ts` with the seeder's `speaker/<meeting>/<label>` and segment IDs, `mock-voiceprints.ts`, `speakers.ts`). Segments: ✅ TRN-01 (`mock-transcripts.ts`, `transcript.ts`; each segment on the speaker `mock-speaker-store.ts` says said it, so renames move lines). People and companies: ✅ ORG-04 (`mock-entities.ts` from the fixtures' attendees, `mock-entity-pages.ts`, `people.ts`, `companies.ts`). | ✅ ORG-01 / ✅ ORG-04 / ✅ ORG-05 / ✅ ORG-06 / ✅ AI-05 / ✅ CAP-04 / ✅ TRN-01 |
| FND-07 | ~~Persist live through the new ports: create the meeting with `MeetingLifecycleRepo::create` (`recording`, `source = live`), append finalized segments with `SegmentWriter::append` (one transaction per batch), stop with `MeetingLifecycleService::transition(StopRecording)` plus `set_timing` (the only writer of `duration_s`) and `set_audio`, and recover with `list_by_status(Recording)`. Then delete `api_save_transcript`'s write through the `transcripts` view.~~ Done by FND-05b: `deino-recording` creates the meeting, saves drafts and batches (`SegmentWriter::save_live`, `append`), stops through `set_timing` + `StopRecording`, recovers at launch; `api_save_transcript` is gone. | ✅ FND-05b |
| FND-07 | Drop the `transcripts` compatibility view and its two triggers (ADR 0012) in a new migration once nothing reads or writes it: `database/repositories/{transcript,meeting}.rs` (search, paging, delete), `audio/import.rs`, `audio/retranscription.rs`, `api_save_transcript`. The card that ports the last caller drops it. TRN-01 reads through `deino-core::transcript`; the inherited session still loads the first page through the view for its metadata. | FND-05b / CAP-08 / CAP-11 / ✅ TRN-01 |
| FND-07 | Legacy imports leave `meetings.source = 'live'` (the default; FND-07 could not tell imported meetings apart). Create imported meetings with `MeetingSource::Import`. | CAP-11 |
| FND-07 | ~~The `transcripts` view hides soft-deleted segments, so a legacy `DELETE FROM transcripts` (re-transcription) leaves them behind.~~ TRN-02's migration makes a delete through the view also remove the meeting's soft-deleted rows. Porting re-transcription to `SegmentWriter::replace` is still CAP-08's. | ✅ TRN-02 / CAP-08 |
| FND-07 | Typed payloads FND-07 left as nullable, unread JSON: ~~`folders.auto_rules_json` (ORG-05)~~ and `action_items.external_refs_json` (INT-03). Each gets a Rust struct and its repository field. ORG-05 dropped the column for the `folder_rules` table, one typed row per rule (ADR 0024). | ✅ ORG-05 / INT-03 |
| FND-07 | ~~`meetings.template_id` names a built-in template as text; make it a foreign key to `templates`.~~ NOT-04's migration makes it a reference (`ON DELETE SET NULL`) and maps the six legacy names to their successors. | ✅ NOT-04 |
| FND-07 | ~~People list aggregates (`people.last_met_at`, `meeting_count`, §3A) were not created: add them with their single writer when the people list sorts by last met; merging people must move speakers, attendees and action items to `merged_into`.~~ Not denormalised: counts and last met are computed on read (39 ms for 2,000 people and 20,000 attendance rows, `list_bench`); merges move every link in one transaction (ADR 0036). | ✅ ORG-04 |
| FND-07 | ~~`calendar_events` has no all-day flag and no per-calendar (sub-account) table; add them with calendar selection in Settings.~~ `calendars` table, `calendar_events.calendar_id` / `is_all_day` (migration `20261011090000`), hidden calendars in `Settings::calendar`. | ✅ ORG-01 |
| FND-04 | ~~`MockAudioSource` playing a fixture's tracks.~~ Done in `deino-audio` by FND-05a: `MockAudio::from_env()` parses `DEINO_MOCK_AUDIO` (`1` = generated voices, otherwise a fixture ID, debug builds only), `TestSignal::from_pcm` resamples a `PcmTrack` to 48 kHz and `MockAudioSource::playing` plays both tracks (`tests/finalize_round_trip.rs` round-trips the standup fixture). What is left is the wiring in `composition.rs`. | FND-05b |
| FND-05a | ~~Wire `deino_audio::Recorder` into the live recording: start it with `CmafEncoderFactory`, `SystemHostClock`, `SystemDiskSpace`; feed each capture source's 48 kHz host-stamped blocks into its `RtSink` (or `MockAudio::from_env()` sources in debug builds); call `find_unclosed` + `recover_recording` at launch before moving `recording` meetings to `processing`; store the recording directory under `app_data_dir`. Remove the 30 s checkpoint saver and IndexedDB transcript recovery once this works.~~ Done by FND-05b (recordings under `<app_data_dir>/recordings/<meeting>`; the checkpoint saver and IndexedDB recovery are deleted). | ✅ FND-05b |
| FND-05a | ~~`finalize_audio` stage (order 10): call `deino_audio::finalize_recording(dir)` and store `mic.m4a`, `system.m4a`, `audio.mp4` in `meetings.audio_*_path`. Finalize is idempotent and writes through temporary names. FND-08 shipped the runner: the stage is a `PipelineStage` in `crates/deino-pipeline/src/stages/` plus one line in `registry::stages`; it needs the recording directory, which FND-05b stores (FND-08 left it out for that reason). `finalize_recording` blocks on AVFoundation, so run it with `spawn_blocking`.~~ Done: `stages/finalize_audio.rs` (repairs first, skips meetings without a recording folder). | ✅ FND-05b |
| FND-05a | ~~Real `AudioSource` adapters: the cpal microphone and the Core Audio process tap push 48 kHz mono blocks stamped with `AudioTimeStamp::mHostTime` (same domain as `SystemHostClock`) through `RtSink::push` only; resample from the device rate on the conditioner side (§3C).~~ Done: `deino-audio::capture` (cpal microphone, process tap, conditioners). | ✅ CAP-02 |
| FND-05a | ~~Show `deino_audio::HealthEvent`s (`Stalled`, `FramesDropped`, `DiskLow`, `StorageFailed`) in the record pill; add the 256 MiB "disk exhausted → stop cleanly" rule.~~ Health notices are toasts and notifications; the open issues (no audio, no transcript, transcription stopped, disk low, disk full) show in the pill and the sidebar, and a disk under 256 MiB stops the recording (`HealthMonitor`, auto-stop's `DiskFull`). | ✅ CAP-10 |
| FND-05a | Anything that decodes Deino's own AAC files with symphonia 0.5 (re-transcription, imports) must trim the 2112 priming frames (`deino_audio::fmp4::AAC_PRIMING`) or decode with ExtAudioFile/AVFoundation. CAP-08's `final_transcription` decodes with `ExtAudioFile` (`deino_recording::NativeDecoder`). | ✅ CAP-08 / CAP-11 |
| FND-05a | CI: `cargo check -p deino-audio` on Linux and Windows (only macOS was built locally; non-macOS paths are `cfg`-stubbed), and run `cargo test -p deino-audio --test kill_harness -- --ignored` on the macOS runner. Re-measure CPU per track in a release build at load < 20 (measured 2.7 % per track in a debug build at load ~99, mock sources included). | FND-03 |
| FND-05a | Not covered: real power loss, WKWebView playback of the fragmented `.m4a.part` files (finalize writes flat files), two recordings at once. cidre a9587fa lacks `finishWritingWithCompletionHandler:`, `AVAssetExportSession` and `ExtAudioFileSetProperty`, so `deino-audio` uses objc2 and a three-function FFI; drop them when cidre gains the bindings. | FND-05b / any cidre upgrade |
| FND-04 | ~~Seed `userNotes` into `meeting_notes` once the notepad reads it, and serve it from the notes mock.~~ `NoteRows` seeds them (converted with `deino-doc`); the `notes` mock serves them from `MockNotes`. | ✅ NOT-01 |
| FND-04 | The meeting page's legacy commands (`api_get_meeting_metadata`, `api_get_meeting_transcripts`; `api_get_summary` went with NOT-02) are answered by the dev:mock legacy shim (`src/dev/mock-ipc/legacy-meetings.ts`); move them into the typed `meetings` mock when the commands are ported, and delete the shim entries. The Transcript tab reads typed commands now (TRN-01); only its inherited tools still call these. | ORG-07 / CAP-08 / CAP-11 / ✅ TRN-01 |
| FND-04 | ~~Run `pnpm test:e2e` and upload `frontend/test-results/` screenshots in CI.~~ FND-03 runs it as the `e2e` step of `scripts/ci-local.sh` and in the Linux `frontend` job (ubuntu-24.04 has Google Chrome), and uploads the screenshots as an artifact. | ✅ FND-03 |
| FND-04 | ~~The sidebar starts collapsed.~~ The sidebar starts open and meetings live on the Meetings page; the smoke tests now go through it. | ✅ FND-10 |
| FND-04 | ~~dev:mock always reports onboarding as completed.~~ `/?onboarding` starts a fresh install's flow (`settings-store.ts`). | ✅ FND-11 |
| FND-04 | AI evals: score notes, action items, insights and chat against each fixture's `groundTruth` (decisions, action items with owner and due date, facts with evidence quotes). Action items: ✅ AI-05 (`extract.action_items` scores precision, recall, owner and due accuracy against `groundTruth.actionItems`). Notes: ✅ NOT-02 (`notes.enhance` in `crates/deino-llm/tests/llm_eval/notes.rs`: verbatim user lines, ≥ 90% cited AI lines, decisions and facts covered). Insights: ✅ AI-06 (`extract.insights` in `crates/deino-llm/tests/llm_eval/insights.rs`: the Decisions template's recall of `groundTruth.decisions` ≥ 0.6 with ≥ 0.9 of answers attributed, and a typed Discovery scorecard). | ✅ NOT-02 / ✅ AI-05 / ✅ AI-06 / AI-02 |
| FND-06 | ~~The `CredentialSource` port reads keys through `SecretStore::get`; delete `summary/credentials.rs` and the model-list commands' stored-key fallbacks.~~ Done: `deino_core::settings::LlmAccess` implements the gateway's credential, endpoint and route ports over the Keychain and `Settings::ai`; `summary/credentials.rs` and the five provider modules are gone. What is left is `api_test_custom_openai_connection` (`api/api.rs`, its own `reqwest` call through `features::secrets::typed_or_stored`): replace it with `plugin:llm\|models` for `custom_openai` with the draft endpoint and key, then delete it and `typed_or_stored`. | FND-09 ✅ / next card touching Summary models |
| FND-06 | ~~The STT code still reads the engine through the string-typed bridge `api::current_transcript_config` (`"parakeetUnified"`/`"parakeet"`/`"localWhisper"`; CAP-01 added `TranscriptionEngine::ParakeetUnified` as the default and kept the bridge, since the legacy engine code it feeds is what CAP-02/FND-05b replace); read `Settings::transcription` (`TranscriptionEngine`) directly.~~ Done: `stt/settings.rs` reads `Settings::transcription`; the bridge is deleted. | ✅ CAP-02 / FND-05b |
| FND-06 | ~~Wire the theme switch (`Settings::ui.theme`) to the app shell.~~ Settings → Appearance and the ⌘K palette save `ui.theme`; `SettingsSync` applies the saved theme, with `localStorage` (`deino.theme`) only as the first-paint cache. The settings screens are tokenised and sectioned; `ModelSettingsModal.tsx` (1.3k lines), `ConfigContext` and the inherited settings panels are still legacy markup: rebuild them as `features/settings/` sections and rows as their cards port them. | ✅ FND-10 (panels: next settings card) |
| FND-06 | ~~The onboarding commands (`onboarding.rs`) keep their legacy snake_case shape.~~ Deleted; progress is a typed settings section and the `onboarding` plugin serves permissions and the sample. | ✅ FND-11 |
| FND-06 | `Settings::privacy` and `Settings::profile` are empty placeholder sections. `profile`: ✅ NOT-05 (name, emails, role, company, focus areas, writing style, context). | PRV-01 / ✅ NOT-05 |
| FND-06 | Transient UI state still uses localStorage (`downloading-models`, `transcription_progress`); move it into Rust-owned state when the model manager and import cards are ported. | CAP-01 / CAP-11 |
| FND-06 | A crash between the first launch's legacy import and the first clean shutdown leaves the dropped keys' zeroed pages in the WAL until the next checkpoint (the import itself checkpoints right after it runs). Consider a `wal_checkpoint(TRUNCATE)` on every open. | FND-05 / FND-07 |
| FND-10 | The sidebar width and collapsed state and the meeting page's right-panel state are not persisted (`features/shell/layout-store.ts`); add them to `Settings::ui` and save them like `ui.theme`. The inherited `ui.transcriptPaneRatio` is unused since the meeting page moved to tabs; drop it or reuse it for the right panel's width. | Next card touching the shell |
| FND-10 | ~~The sidebar's live record row cannot stop a recording.~~ It has a hover Stop; the record panel has one too. | ✅ CAP-10 |
| FND-10 | Set each Tauri window's native background to `--bg` for its theme and show windows after the first paint (no white flash in dark mode). ~~Broadcast theme changes to the `record_panel` window.~~ The panel follows the main window's saved theme through the `storage` event (CAP-10); its window is transparent. | FND-11 / ✅ CAP-10 (panel) |
| FND-10 | `legacy-files.mjs` lists `src/components/**` as one glob, so a ported file (for example `EmptyStateSummary.tsx`, rewritten on the primitives) cannot leave the list on its own. Expand the glob into explicit files. | FND-03 / next card porting a component |
| FND-10 | Home is still the inherited record screen. Build Home (Coming up, Recent notes, empty state with shortcuts) per ui.md §4.1. | ORG-02 |
| FND-10 | The meeting header shows the date chip and resting "Add attendees" / "Add tag" chips; fill them, ~~add the Share/Export menu~~ (EXP-01: Share in the meeting title bar, per ui.md §4.2) and ~~the Enhance button~~ (NOT-02: Enhance/Regenerate in the meeting title bar, ⌘⇧E). Tags: ✅ ORG-06 (tag chips and the "Add tag" combobox; ORG-05 added the folder chips and "Add to folder"). | ✅ ORG-04 (attendee chips with hover cards, add and remove) / ✅ ORG-06 / ✅ EXP-01 / ✅ NOT-02 |
| FND-10 | The Notes and Transcript tabs render the inherited panels through `features/meeting/legacy/` (`panels-context.tsx`). Replace each slot in `features/meeting/slots.ts` and delete the bridge: the Enhanced / My notes `SegmentedControl` (NOT-01/NOT-02) and transcript bubbles with speaker colours (TRN-01). NOT-01 added the Summary / My notes switch (`slots/notes-slot.tsx`, ⌘E) and "My notes"; NOT-02 replaced Summary with "Enhanced" (`features/notes`), so only the Transcript tab still uses the bridge (now transcript-only). TRN-01 replaced the Transcript slot (`slots/transcript-slot.tsx`) and deleted `slots/legacy-slots.tsx`; the bridge now only lends it the inherited "open recording folder" and "re-transcribe" buttons (`transcriptTools`). | ✅ NOT-02 / ✅ TRN-01 / CAP-08 / CAP-11 |
| FND-10 | The inherited settings panels (General, Recording, Transcription, Summary models, Beta) are tokenised but keep their own card markup. Rebuild them on settings sections and rows (ui.md §3 Settings layout) as their values move into the settings store. | FND-06 |
| FND-10 | ~~dev:mock does not answer `api_delete_meeting`, so deleting a meeting fails there (the real app works). Answer it in the legacy shim, or in the typed mock once ported.~~ Resolved by ORG-07: the legacy delete is gone, and dev:mock answers the typed trash, restore and delete-forever commands (`mock-meetings-list.ts`). | ✅ ORG-07 |
| FND-09 | ~~The gateway's HTTP goes through the temporary `ReqwestTransport`.~~ FND-12 gave every provider a `deino-net` scoped client (`llm/http.rs` `NetTransport`). The eval harness keeps its own `reqwest` dev-dependency (`crates/deino-llm/tests/llm_eval/live.rs`); move it onto `deino_net::ReqwestNetClient` with a shared transport when convenient. | ✅ FND-12 (eval harness: next card touching `deino-llm`) |
| FND-09 | Usage page: list and sum `llm_calls` (per route, provider and model over time; tokens, latency, failures). Per provider and model with tokens, failures and an estimated cost: ✅ ORG-09 (Stats → LLM usage). Still open: per route, latency, and usage over time. | ✅ ORG-09 (partly) / a small usage card |
| FND-09 | The legacy summary's `reasoning_stripped` flag now reflects only `<think>` envelopes removed from text: providers drop structured reasoning (`reasoning_content`, Anthropic thinking blocks) before it reaches the app, so the "reasoning was filtered" toast no longer appears for those. Moot: the legacy summaries, the flag and the toast are gone. | ✅ NOT-02 |
| FND-09 | Ollama: the legacy `ollama list` CLI fallback (when the HTTP API failed) and the per-pull duplicate guard in Rust were not ported; pulls and deletes use the saved endpoint (the form's unsaved endpoint only applies to model lists). | Next card touching Summary models (FND-11 did not touch them) |
| FND-09 | ~~The local embedding model has no download yet.~~ AI-01 added it to the catalog (`<models>/embedding/nomic-embed-text-v1.5.Q8_0.gguf`, verified download through the embedding `ModelService`); `SidecarLocalModels` resolves the file through it and starts the sidecar for `embed`. Verified end to end with the real model by `llm::retrieval_eval`. | ✅ AI-01 |
| FND-09 | llama.cpp's own load logs (thousands of `create_tensor` lines per model load) still reach the app's stderr; route them through `llama_cpp_2::send_logs_to_tracing` or filter them. The helper's own diagnostics carry only counts and timings. | Next card touching llama-helper |
| FND-09 | The real-app click-through of Settings → AI was done in `pnpm dev:mock` only: the dev binary run with the throwaway identifier started, migrated (`llm_calls`) and wired the gateway, but the computer-use tools cannot drive an unbundled dev binary. The gateway, sidecar adapter and llama-helper were exercised end to end by `llm::smoke_tests` (streaming, grammar-structured output, a full legacy summary with Gemma 3 4B). | FND-03 (a signed debug bundle for UI smoke runs) |
| FND-09 | `CLAUDE.md`'s crate table does not list `deino-llm` yet; add the row (gateway, routes, providers, prompts, `MockProvider`). | Orchestrator |
| FND-09 | Settings → AI lists providers with masked keys but keys are still entered in the inherited Summary models dialog (`ModelSettingsModal.tsx`); move key entry and the Ollama/custom endpoints onto the AI page and retire the dialog's provider pickers in favour of the route table. | NOT-04 / next settings card |
| FND-13 | The real-app UI click-through of the sidebar rename was not completed (the background UI session was interrupted); the IPC path is covered by contract tests and the app was verified to start and migrate an existing database. FND-03 (CI only) did not take it. | Next card touching the sidebar (FND-10) |
| FND-08 | ~~Recording does not reach `processing` through `MeetingLifecycleService` yet, so the pipeline only runs from `rerun` (the strip's retry, or a meeting left `processing`). When the recorder stops with `transition(StopRecording)`, the pipeline's `legacy_summary` stage summarises the meeting: remove the meeting page's auto-summary on stop (`useSummaryGeneration` with `source = 'recording'`) in the same PR so it does not run twice, and have the stage honour `Settings::ai.auto_summary` (skip with a reason when off; a manual re-run still runs).~~ Done: stop transitions to `processing`; the meeting page's auto-summary is removed; `legacy_summary` skips with a reason when `auto_summary` is off. | ✅ FND-05b |
| FND-08 | `legacy_summary` reports every failed legacy summary job as retryable: `SummaryJob` records only a message on `summary_processes` (which may quote the provider, so it stays in the summary panel). Return the `deino_core::summaries::SummaryError` (with `LlmError::retry_after`) from the job so the stage can tell a missing key or model (permanent) from a rate limit (retryable). Done for `enhance_notes`: `EnhanceError::is_retryable` (a missing key or model is permanent, with a user-safe message; a rate limit or overload retries). | ✅ NOT-02 |
| FND-08 | Replace `legacy_summary` (order 50) with `enhance_notes`: delete the stage, the `Summarizer` port and the app adapter; startup prunes the old jobs. The legacy summary panel does not notice a summary the pipeline started until it is reopened; the new notes view should read the stage's status. Done: the stage, port and adapter are deleted; the Enhanced view shows "Writing your notes…" while the stage runs (`useIsEnhancing` reads the pipeline status). | ✅ NOT-02 |
| FND-08 | `PipelineService::cancel(meeting)` has no command. Cancel a meeting's pipeline when it is trashed or deleted, and offer Cancel next to a running stage if a stage turns out to be long (transcription re-pass). ORG-07: trashing a meeting cancels its running stage (the `MeetingWork` port, ADR 0046); Cancel next to a running stage is still open. | ✅ ORG-07 (trash) / CAP-08 |
| FND-08 | The processing strip sits under the meeting header's chips as a plain row of stages. Give it its final place and look (a quiet header line or the Details panel; ui.md has no spec for it yet), with a loading skeleton, and add a Playwright check of the retry flow (dev:mock serves the one-on-one with a failed summary). | FND-10 follow-up / next card touching the meeting page |
| FND-08 | Real-app smoke ran the dev binary under a throwaway identifier against a seeded database: startup pruned a retired stage, recovered a crash-left `running` job (started "in the future", the clock-skew case), skipped meetings without transcript, failed the one with no summary model, and moved each meeting to `ready`/`failed`. Not done in the real app: clicking the strip's Retry (verified in `pnpm dev:mock`; covered by contract tests) and a summary generated end to end by the stage with a configured model. | FND-03 (signed debug bundle for UI smoke runs) |
| FND-08 | Each new stage adds its readable name to `frontend/src/features/pipeline/copy.ts` (`stageLabels`) and its fixture state to `src/dev/mock-ipc/pipeline.ts`; otherwise the strip shows the raw stage ID. | Every card that adds a stage |
| FND-03 | Coverage is not reported in CI yet (`cargo llvm-cov` for the portable crates is cheap on Linux; Vitest coverage needs `@vitest/coverage-v8`). | The next card that touches CI |
| FND-03 | Branch protection that requires `rust (portable crates)`, `rust (app on Linux)`, `frontend` and `pr title` needs GitHub Pro (or a public repo) for a private personal repository. Until then, `scripts/ci-local.sh` before every merge is the gate. | Owner |
| FND-03 | Local `pnpm run tauri:build` in a checkout under `~/Documents` can still fail at `codesign` (provenance and File Provider attributes). Reuse `release-macos.yml`'s `beforeBundleCommand` override in a `tauri:build:dmg` script, or build outside `~/Documents`. | Next card that ships a local build script (or FND-05) |
| FND-03 | `.github/verify-portable-ggml.cjs` and its test are no longer run by any workflow (the Windows distribution workflows were upstream's); `docs/BUILDING.md` documents running them by hand. | Delete with the Windows build story, or when a Windows job returns |
| FND-03 | `scripts/ci-local.sh` runs its steps one after another. The frontend steps (about 80 s) could run beside the Rust compile once the machine is not shared by many agents. | Any later CI change |
| CAP-01 | ~~Persist the live lane's segments with their timed words (`FinalSegment.words` → `transcript_segments.words_json`, `deino_stt::Lane` → `Channel`, `deino_stt::Millis` (u64) → `deino_domain::Millis` (u32)) from Rust, instead of the legacy `transcript-update` event and the frontend save.~~ Done: the router saves each lane's segments with words, channel and speaker. | ✅ FND-05b |
| CAP-01 | ~~Split the live lane into microphone and system lanes: `EngineSlot::Live(Lane::Microphone/System)` already run in separate workers; feed each from its own capture track and label lines Me/Them.~~ Done: Me and Them lanes, one worker each. | ✅ CAP-02 |
| CAP-01 | On a CPU fallback the live stream cannot keep up (spike: RTF 1.05 at the default window). Switch that lane to batch-after-utterance (VAD-bounded runs) or the 1040/1040 window, and raise a recording-health warning (the runtime logs `live transcription runs on the CPU`). CAP-10's health monitor warns when a lane loses its stream ("Transcription restarting") or the microphone speaks for 30 s without a line, which catches a lane that stops keeping up but not a slow one. | CAP-08 / CAP-10 |
| CAP-01 | Offer "Responsive" (5600/560/320, default) and "Accurate / low power" (5600/1040/1040) live windows in typed settings; read the window menu from the GGUF (`stt.parakeet.encoder.att_chunk_*_choices`) instead of probing a stream. | FND-06 |
| CAP-01 | ~~Replace `stt/http_source.rs` with `deino-net`.~~ FND-12: `NetByteSource` over `net.approved_download(HfModels)`; Whisper, Parakeet TDT and the built-in model downloads too. | ✅ FND-12 |
| CAP-01 | ~~Onboarding's "Transcription Engine" step: rewrite it with the typed API and show the NVIDIA notice.~~ The AI setup step downloads through the `transcription` plugin with a progress bar and shows the NVIDIA notice. It still shows no download speed. | ✅ FND-11 (speed: open) |
| CAP-01 | `build.rs` and `docs/GPU_ACCELERATION.md` still describe whisper.cpp/CoreML features; `coreml` and `openblas` are now no-op features kept for scripts; `.github/*-portable-ggml*` and `scripts/dev-env.sh` still mention whisper.cpp. Clean them up. | The next card that touches CI or the build |
| CAP-01 | The Parakeet TDT (ONNX) and Whisper engines remain under "Advanced" with their legacy managers; Parakeet TDT v3 also exists as a transcribe-cpp GGUF (`handy-computer/parakeet-tdt-0.6b-v3-gguf`), which would let `ort` go once diarization no longer needs it. | CAP-03 / later cleanup |
| CAP-01 | Batch fallback restarts a lost stream at the loss point, so a word spoken across that instant can be split; cut at the next decoded silence instead. | CAP-02 |
| CAP-01 | A legacy install whose old database had no transcription row imports as the new default (Parakeet Unified), which needs a model download before the first recording; recording start reports the missing model. Consider keeping such installs on Parakeet TDT when its model is already on disk. FND-11 left the import alone: onboarding's AI step offers the download. | Open |
| CAP-01 | Per-process CPU accounting for the app and its STT workers (`EnginePool::worker_pids`), needed for CAP-02's "≤ 1.6× single lane" and CAP-08's 15% acceptance. CAP-02 measured with `stt_bench` and `ps` sampling instead (ADR 0026); the in-app accounting is still open. | CAP-08 |
| FND-12 | Delete the legacy `api_test_custom_openai_connection` (it tests an unsaved endpoint with its own `reqwest` client, outside the egress log) in favour of Settings → AI's route test after saving the endpoint; then drop `reqwest` from the app's `[dependencies]` and `clippy::disallowed_types` from `legacy_lints.rs`. | NOT-04 / next settings card |
| FND-12 | Offline mode, or a switched-off provider, reaches the gateway as `HttpError::Connect` with a user-safe reason; route notes to the built-in model instead of failing (check `network.offlineMode` in the `Router`, or add a typed `HttpError::Blocked`). | Next card touching `deino-llm` |
| FND-12 | The built-in model downloads (`summary_engine/model_manager.rs`) go through `deino-net` but have no pinned size or SHA-256 and keep their own resume loop; pin both and switch to `deino_net::HttpDownloader`. The Parakeet TDT and Whisper downloads likewise keep their legacy loops. | Next card touching the model managers |
| FND-12 | ~~Connecting an integration switches its `ServiceId` on through `ServiceSwitch::set_enabled` (and disconnecting switches it off); providers receive `net.scoped(..)` from the composition root.~~ Done through the `Egress` port (`NetEgress`): connect switches on, the last disconnect switches off, offline mode stops connects and deliveries before anything is sent. | ✅ INT-00 |
| FND-12 | Webhook URLs must add their hosts in `SettingsServicePolicy::custom_hosts` (`composition/network.rs`), like the custom LLM endpoint. | DEV-01 |
| FND-12 | The egress log keeps the newest 10,000 rows; add it to retention settings and a "Clear log" action. | PRV-01 |
| FND-12 | The Network page's `SettingsSection`/`SettingsCard` (`features/network/components/settings-section.tsx`) are the ui.md settings section and card; promote them to `ui/` when the inherited settings pages are rebuilt on them. | FND-10 follow-up (settings panels) |
| FND-12 | Acceptance "a full meeting with no services connected makes zero outbound connections" was checked on a fresh smoke build (`com.xenshreyas.deino.smoke-fnd12`) from launch to idle only: `lsof -i` and `nettop` showed no sockets. Repeat it for record → notes with the local model → export once FND-05b lands. | FND-05b |
| CAP-03 | The `diarize` stage downloads its 32 MB of models on the first meeting it processes (retried, then skipped so notes still run); offer the download in the model manager and onboarding instead, so the first meeting is not the trigger. FND-11 did not add it: the diarization models have no download command yet. | Model manager UI, then onboarding's AI step |
| CAP-03 | Attribute WeSpeaker ResNet34-LM (CC-BY-4.0, text in `THIRD_PARTY_NOTICES.md`) on the About screen. | FND-10 / the About screen's next card |
| CAP-03 | Drive `deino_diarize::online::LiveSpeakerTracker` from the system lane: embed finalized utterances ≥ 1.5 s off the lane thread with `Diarizer::embedder()` (or an `EmbeddingExtractor`), label live speakers, and let the post-meeting pass replace them (map live renames by word overlap). Not done by CAP-02; carried below. | CAP-03 / CAP-10 |
| CAP-03 | ~~Use the stored centroids (`SpeakerRepo::voice`), `deino_diarize::voiceprint` (`pick_unique`, `complete_by_elimination`, `select_speaker_spans`) and `KnownSpeaker`s in `DiarizeRequest` (add them to `TrackRequest`) for `identify_speakers`; link "Me" to the `people.is_me` row.~~ Done by CAP-04 (ADR 0029): the primitives moved to `deino-core::voiceprints`, `TrackRequest::known` carries the attendees' voiceprints, and the microphone speaker is the `is_me` person. | ✅ CAP-04 |
| CAP-03 | Re-tune the 0.7045 threshold and re-time on real meetings: run `diarize-bench file` on AMI dev / VoxConverse RTTMs (not downloaded here) and on an idle M1; every DER so far uses synthetic voices (`docs/perf/diarization.md`). CAP-04 did not re-tune it (no real recordings were authorised). | A tuning pass |
| CAP-03 | Decoding a 60-minute track holds about 230 MB of 16 kHz samples while it runs; add a file-backed `AudioSource` (`ExtAudioFileSeek`) if memory matters. | Later cleanup |
| NOT-04 | ~~Register the `/template` command in the TipTap notepad: `Extension.create({ addProseMirrorPlugins() { return [Suggestion({ editor: this.editor, ...templateSlashSuggestion(deps) })] } })` with `deps = { templates: () => cached list, setMeetingTemplate }` (`features/templates`, `slash/template-slash.ts`) and render `TemplateSlashList` in the suggestion popover. The module is TipTap-free (structural types), tested with a fake editor.~~ Registered in `features/notepad/editor/extensions.ts` (`TemplateCommand`). | ✅ NOT-01 |
| NOT-04 | Generate notes with `MeetingTemplateService::auto_choice` (rules → default → `tagging` classifier → General), record the template used in `enhanced_notes.template_id`, and consider persisting an automatic choice so the classifier runs once per meeting. Retire the legacy summary toolbar's own template dropdown (it now writes the meeting's template through `useLegacyTemplateChoice`). Done: `TemplateSource` over `auto_choice`, `enhanced_notes.template_id`, the dropdown and hook removed; persisting an automatic choice is a new row below. | ✅ NOT-02 |
| NOT-04 | Export copies a template's JSON to the clipboard (WKWebView has no reliable `<a download>`); add a native save dialog when a dialog plugin is on the JS side. | Next card touching file dialogs |
| NOT-05 | When the profile's first address belongs to an existing person, that person becomes "Me" and the previous "Me" row stays as a plain person; merge the two (attendance, speakers, action items) with ORG-04's merge. ORG-04's merge carries "Me" to the survivor and moves every link; the user merges from the person page. Merging automatically when the profile is saved is still open. | ✅ ORG-04 (manual) / NOT-05 (automatic) |
| NOT-05 | `profile::InternalDomains` is the shared internal-vs-external rule: fill `calendar_events.is_external` with it (✅ ORG-01: the profile's addresses plus the user's own invitation entries; one public-domain list, `companies::PUBLIC_EMAIL_DOMAINS`), and use it for follow-up recipients and briefs. | ✅ ORG-01 / NOT-07 / NOT-08 |
| NOT-04 | The two follow-ups FND-09 and FND-12 assigned to "NOT-04 / next settings card" (move key entry onto Settings → AI; delete `api_test_custom_openai_connection`) are settings work outside the templates card and remain open. | Next settings card |
| NOT-01 | ~~The record screen's notepad writes into a `scheduled` draft meeting (created on the first edit) and `adoptLiveNotes` moves it into the meeting the legacy stop path saves (`notes\|adopt`, one call in `hooks/useRecordingStop.ts`). Once Rust creates the meeting at recording start, point `LiveNotepad` at it and delete `live-notes.ts`, `NoteService::adopt` and `NotesRepo::transfer`.~~ Done: the record screen's `Notepad` writes into the meeting being recorded; `live-notes.ts`, `NoteService::adopt`, `NotesRepo::transfer`, `NoteImages::relocate` and `notes|adopt` are deleted (`create_scheduled` stays). | ✅ FND-05b |
| NOT-01 | Opening an upcoming event's note: call `notes\|create_scheduled` with the event's title and link it with `MeetingLifecycleRepo::link`; ⌘N "New note" still opens the record screen, whose notepad makes a draft. | ORG-01 / ORG-02 |
| NOT-01 | The record panel's "Open notes" brings the main window forward on the record screen (CAP-10). Opening `NotepadWindow` (exported by `features/notepad`) in its own window on the live meeting is still open; `plugin:notes:notes-saved` lets the two notepads tell their saves apart. | Later (NOT-01 / CAP-10) |
| NOT-01 | TipTap's `code` mark excludes every other mark, `origin` included, while `deino-doc` lets code carry provenance; extend the Code extension's `excludes` when AI text first carries inline code. ~~Replace `SummaryDocument` with the enhanced notes.~~ Done (NOT-02). The `code` mark's `excludes` is still open: the notes prompt asks for plain text, so no AI run carries inline code yet. | NOT-02 ✅ / the first card whose AI text carries code |
| NOT-01 | ~~Deleting a meeting (legacy `api_delete_meeting`) leaves its `<app data>/meetings/<id>/images/` folder; delete it with the meeting (and keep it while in the trash).~~ Resolved by ORG-07: the folder stays while the meeting is in the trash; "Delete forever" and the 30-day purge delete `meetings/<id>` and `recordings/<id>` (`DiskMeetingFiles`). | ✅ ORG-07 |
| NOT-01 | ~~Index `meeting_notes.notes_markdown` in `search_fts`.~~ Indexed by trigger as kind `note` (and embedded by note section). | ✅ ORG-08 |
| NOT-01 | Dragging an image over the window still shows the audio-import overlay (only the drop is classified); hide it when every dragged path is an image. ⌘⇧V reads `navigator.clipboard`, which WKWebView may refuse: switch to the clipboard plugin if Manual QA shows it. | Next card touching the shell |
| NOT-01 | React StrictMode stays off (`main.tsx`): TipTap is safe, but the legacy contexts' mount effects are not idempotent. | Whoever ports the last legacy context |
| CAP-06 | ~~Recording starts through the shell's start path (`ShellRecordingStarter` emits `RecordingRequested`; the record screen starts the legacy recorder) and "recording" is the legacy `IS_RECORDING` flag (`LegacyRecordingStatus`). Switch both to the typed `recording.start` / recording state: start with the detection's app and linked event as the meeting title, and open that meeting's notepad (NOT-01) instead of the record screen.~~ Done: `features/detection/recorder.rs` starts the live recorder titled after the linked event or "<App> meeting" and reports its state; "Take notes" opens the record screen, which shows that meeting's notepad. | ✅ CAP-02 |
| CAP-06 | ~~Calendar association ships as `NoCalendar`. Implement `CalendarLookup` over `calendar_events` with `deino_detect::calendar::pick_linked`.~~ `features::calendar::SyncedCalendar` over `CalendarService::events_near` (in memory, refreshed by every sync), skipping all-day and declined events. Still open: let the prompt's event line open the event (needs ORG-02's event view). | ✅ ORG-01 (link) / ORG-02 (open) |
| CAP-06 | Browser meetings are recognised by URL only when Accessibility is granted (`reads_browser_urls`); otherwise a browser call prompts after the 15 s hold as "Meeting detected". Request Accessibility with an explainer and a deep link to System Settings, and add the ScriptingBridge active-tab fallback (Muesli) if AX misses Chrome tabs. | CAP-05 (✅ the explainer and deep link: Settings → Speaker tags; the ScriptingBridge fallback is still open: later) |
| CAP-06 | ~~Auto-stop: CAP-07 needs the recorded meeting's app and its release (20 s) while recording.~~ `DetectionService` keeps reporting which meeting app holds the microphone while recording (`MicrophoneHolders`); `AutoStopService` is armed with the detected meeting's app. | ✅ CAP-07 |
| CAP-06 | Detection ignores Do Not Disturb (`notifications.respectDoNotDisturb`) and has no "always detect" include list (anarlog); add both if users ask. The prompt window re-reads the theme cache on each detection only; broadcast theme changes to it with the record panel's. | CAP-10 / ORG-02 |
| CAP-06 | Real-call smoke not observed by the agent: the smoke build (`com.xenshreyas.deino.smoke-cap06`) started detection with all three listeners and read 43 to 46 Core Audio processes, but `ffmpeg` from the agent's shell could not hold the microphone (macOS permission prompt for the host app) and computer-use access to Photo Booth was declined. Run the Manual QA list in `crates/deino-detect/README.md` (Zoom, Meet in Chrome, Slack huddle, ignore list, auto-start). | Owner |
| CAP-06 | `src/dev/mock-ipc/mock-database.ts` imports `@/features/meetings`, whose generated mock imports `database.ts` back; an entry that does not load the meetings feature first hits a TDZ error in `pnpm dev:mock` (worked around in `src/detection-prompt.tsx`). Move `asMeetingId` out of the cycle. | FND-04 owner / next card touching mock IPC |
| ORG-01 | Manual QA on a real Mac (the agent may not grant OS permissions): turn sync on in Settings → Calendar, accept the macOS Calendars prompt, check Coming up and a calendar's visibility switch, add an event in Calendar.app (it appears within a few seconds), sleep and wake, then deny access in System Settings and check the explainer and the "Open System Settings" link. The ignored `deino-eventkit` test `live_store` prints counts after the grant. | Owner |
| ORG-01 | The Google and Microsoft connectors need the owner's app registrations (§9; `docs/setup/google-calendar.md`, `docs/setup/microsoft-graph.md`); their sign-in and sync are covered by fakes only. Google in "Testing" expires refresh tokens after 7 days. | Owner |
| ORG-01 | ORG-02 builds on `calendar\|upcoming` and `CalendarEvent::record_override` (syncs never write it; add the command that sets it), and opens an event's note with NOT-01's `create_scheduled` + `MeetingLifecycleRepo::link`; `meetings.calendar_event_id` stays the only link. | ORG-02 |
| ORG-01 | ~~INT-00 adopts `deino_connect::oauth`.~~ INT-00 went the other way: `deino-integrations::oauth` is the one OAuth layer (redirect modes, client auth, revocation, device code), and `deino-connect`'s calendar connectors now sign in and refresh through it (`calendar/oauth.rs` holds their `OAuthSpec`s); its own `oauth` module is gone. | ✅ INT-00 |
| ORG-01 | One account per connector (one refresh token per provider); several Google or Microsoft accounts need a token per account and an account picker. Google and Graph all-day events are stored at the Mac's local midnight; a time-zone change re-syncs them. Graph's series key is `seriesMasterId`, EventKit's and Google's the iCalendar UID. | Later, if asked |
| ORG-01 | Turning sync on prunes fixture accounts the Rust seeder wrote under the `apple` provider (they are not in the snapshot); seed fixtures under a provider no source syncs if dev builds need both. | Next card touching fixtures |
| ORG-01 | `deino-eventkit` compiles to `Unsupported` stubs off macOS; confirm on the Linux CI job, which this card did not run. | FND-03 follow-up |
| ORG-01 | The connectors section's Vitest imports `features/settings/settings-fixture.ts` by relative path; expose a shared settings test fixture if more features need one. | Next card touching settings tests |
| ORG-06 | Pre-tag upcoming calendar events (the card's stretch goal): run the auto-tagger on an event's title, description and attendees when ORG-02's Coming up list shows it, and carry the tags onto the meeting it becomes. | ORG-02 |
| ORG-06 | The auto-tagger reads a meeting's summary from the legacy `summary_processes.result` (Markdown), else the transcript's opening (`tags/digest_repo.rs`). Read the enhanced notes instead when NOT-02 replaces `legacy_summary`. Done: `digest.sql` and `examples.sql` read the newest `enhanced_notes.markdown`. | ✅ NOT-02 |
| ORG-06 | Bulk tagging (select many meetings, add or remove a tag) on the Meetings page. ORG-07: the bulk bar adds a tag to the selection (`tag_many`); `untag_many` exists but has no UI yet (see the ORG-07 row below). | ✅ ORG-07 (add) |
| ORG-05 | The folder page keeps a quiet placeholder for folder chat ("Ask about this folder") and for integrations; wire them when those cards land. | AI-02 / AUT-01 |
| ORG-05 | Auto-add rules apply to meetings processed after the rule exists; offer "Also add the N past meetings that match" when a rule is created. | Next card touching folders |
| ORG-05 | The sidebar tree's open and collapsed folders live in a zustand store (`features/folders/tree-store.ts`) and reset on reload; persist them with the sidebar state in `Settings::ui` (see the FND-10 row above). | Next card touching the shell |
| ORG-05 | The Rust `FixtureSeeder` (Settings → Developer) does not seed folders, tags or folder rules; only dev:mock has them (`dev/mock-ipc/organizer.ts`). Seed the same rows with the same `fixtureUuid` IDs. | Next card touching fixtures |
| ORG-05 | ~~`pnpm test:e2e` reuses any server already listening on 3119 (`reuseExistingServer`), which can be another worktree's dev:mock when agents run in parallel; give each run its own port (`MOCK_PORT` from the environment).~~ Done by CAP-02: `MOCK_PORT` overrides the port in `vite.config.ts` and `playwright.config.ts` (`MOCK_PORT=3149 scripts/ci-local.sh e2e`). | ✅ CAP-02 |
| EXP-01 | The AI notes in exports are the legacy summary's Markdown (`summary_processes.result`); render NOT-02's enhanced document through `deino_core::export::DocNoteRenderer` like the notepad, and decide whether rich copies keep the AI/user provenance colours. The inherited Notes panel's copy button now copies the stored summary, not unsaved edits in the editor. Done: `DocNoteRenderer` reads the newest version through `EnhancedNotesReader`; the Enhanced view's Copy menu uses EXP-01's copy (Markdown, rich text). Rich copies do not keep the provenance colours (open). | ✅ NOT-02 |
| EXP-01 | ~~The Insights section exists in `ExportSections` and `Settings::export` but renders nothing and has no toggle (`SECTION_KEYS`) until there are insights.~~ Every format renders it (`deino_export::insights`), the dialog and Settings → Export have the toggle, and the account export adds `insights.csv`. | ✅ AI-06 |
| EXP-01 | PDF glyphs outside Inter's latin and latin-ext subsets (emoji, CJK, Greek, Cyrillic) render as missing glyphs: add a fallback font to `crates/deino-export/fonts/` (Noto Sans subsets, OFL) or read the system fonts in `pdf/world.rs`. | Later polish |
| EXP-01 | Exports carry `deino://meetings/<id>` links (frontmatter `link`, JSON `link`, CSV); they open nothing until the URL scheme is registered. | DEV-05 |
| EXP-01 | Not verified in a bundled app: the save sheet attached to the main window, pasting a rich copy into Gmail, Google Docs and Slack, the Mail draft from `mailto:`, the folder picker. Contract tests cover the commands and Playwright the UI over `dev:mock`. | FND-03 (signed debug bundle for UI smoke runs) |
| EXP-02 | Auto-export writes a renamed meeting under its new name and leaves the old note (it never deletes the user's files). Keep a per-meeting record of the last path if moving notes is wanted. It runs only as the last pipeline stage, so meetings processed before it was turned on (or whose pipeline stopped on a permanent failure) are not in the vault: add "Export all meetings to the folder now" to Settings → Export. ORG-07 added bulk export: ⌘A on the Meetings page, then Export, writes every selected meeting into a folder the user picks (`ExportService::save_many`, single-export names and default sections). A Settings → Export button that runs the auto-export templates over every meeting is still open. | Later |
| EXP-02 | The account export includes one recording per meeting (`audio_path`, else the microphone track), stored uncompressed in the zip; per-track files and video are left out. | CAP-09 / later |
| EXP-02 | `dev:mock`'s pipeline mock (`src/dev/mock-ipc/pipeline.ts`) still models only `legacy_summary`; add `diarize` and `auto_export` fixture states. NOT-02 moved it to `enhance_notes` (with `auto_title`, `auto_tag`, `auto_folder` rows and a failed run on the support call); `diarize` and `auto_export` states are still missing. | Next card touching the pipeline strip |
| FND-05b | The CPU target "≤ 1.6× single lane" was not confirmed: `stt_bench` on the 54 s fixture gave 1.13×–3.85× at load averages 14–38 from parallel agents (ADR 0026). Re-measure on a quiet machine with the full app (both workers plus AEC); if over, consider a VAD-gated Them lane or a shared worker. | CAP-08 |
| CAP-02 | `sonora` 0.2.0 allocates about 35 small vectors per 10 ms AEC frame (pinned at 36 by `tests/aec_allocations.rs`); it runs off the RT threads, but upstream a scratch-buffer patch and drop the bound to 0. | Any audio card |
| CAP-02 | Live speaker labels: drive `deino_diarize::online::LiveSpeakerTracker` from the Them lane (embed finalized utterances ≥ 1.5 s off the lane thread) and map them onto the post-meeting speakers. Not wired: Them is one speaker until `diarize` runs. | CAP-03 / CAP-10 |
| CAP-02 | The process tap excludes Deino's own PID only; audio the WebKit content process plays (a video in the notepad) is captured as Them. Exclude the WebContent helper's process object too. | CAP-10 |
| CAP-02 | Settings → Recording still shows "Save audio recordings" and the `~/Movies/Deino` save location; live recordings always save under `<app_data_dir>/recordings/` and `recording.autoSave` is ignored. Remove or repurpose them (import and re-transcription still use the old folder). | CAP-10 / EXP-02 |
| CAP-02 | Not tested on hardware here: AirPods connecting mid-call, a USB microphone unplugged, sleep and wake during a recording, the first-run permission prompt (see the manual QA list in the CAP-02 PR). `tap_probe` showed the global tap keeps capturing across output changes. | Manual QA before release |
| CAP-02 | The batch fallback still restarts a lost stream at the loss point (CAP-01 row above); not changed here. | CAP-08 |
| CAP-02 | dev:mock's legacy meeting mock (`legacy-meetings.ts`) knows only fixture meetings, so a meeting recorded in dev:mock logs "Meeting not found" on its page; answer empty for unknown IDs. | Next card touching dev:mock |
| CAP-02 | Unused inherited dependencies in the app crate: `dashmap`, `rand`, `dasp`, `core-graphics`, `objc` (no source file names them). FND-05b removed `ringbuf` and `futures-channel`. | FND-13 follow-up |
| NOT-02 | ~~The citation popover's "Show in transcript" navigates to `?tab=transcript&segment=<id>`; the Transcript tab does not read `segment` yet.~~ The Transcript tab scrolls to and marks that segment, then drops `segment` from the URL (TRN-01). | ✅ TRN-01 |
| NOT-02 | ~~Play in the citation popover is disabled.~~ It plays the meeting's recording from the excerpt's start (`playerActions.playFrom`), disabled ("No recording to play") without one. | ✅ TRN-03 |
| NOT-02 | ~~The prompt's `<vocabulary>` block is fed by the `Vocabulary` port with `NoVocabulary`; implement it over TRN-05's custom vocabulary.~~ `MaterialSources::sqlite` feeds `vocabulary::PromptVocabulary` (the user's terms; names already reach the prompt as attendees and speakers). | ✅ TRN-05 |
| NOT-02 | Enhancing a meeting whose template is "Auto" runs the `tagging` classifier each time; persist the automatic choice (`meetings.template_id` or a cache) so it runs once per meeting. | NOT-04 follow-up / next card touching templates |
| NOT-02 | `Settings::ai.auto_summary` ("Generate notes automatically") keeps the legacy name and its default (on); rename the field (a settings migration) and decide the default for users without a key or local model, where every automatic run fails with "Add an Anthropic key…". | Next card touching Settings → AI |
| NOT-02 | With automatic notes off the stage is skipped and the Enhanced view offers Enhance; there is no "re-run the notes stage" in the processing strip for that case (its rerun runs the skipped stage, which skips again). | Next card touching the pipeline strip |
| NOT-02 | AI-05 merged while NOT-02 was in review: the enhanced notes' `todo` blocks are plain `taskList` items, not yet linked to action items (AI-05's row below: `useActionItemTodos`, `todoOf`, `useUpdateActionItem`); the Notes tab keeps `MeetingActionItems` above the notes until then. | NOT-02 follow-up |
| NOT-02 | `NoteService::markdown` and the notepad's stored Markdown are unused by the notes engine (it reads the TipTap document); keep or drop when the next card touches `deino_core::notes`. | Next card touching `deino_core::notes` |
| NOT-02 | The template editor's prompt preview shows the template's sections and instructions only; show the full rendered `notes.enhance` prompt for a chosen meeting. | Next card touching templates |
| NOT-02 | Live `DEINO_EVAL=1 pnpm eval notes.enhance` was not run here (no provider key in this environment); run it with a key and record the scores (and any threshold change) in `docs/perf/`. | Next card touching the notes prompts |
| NOT-02 | Not verified in a bundled app: a real recording through the `enhance_notes` stage with a hosted and with the local model, the legacy-summary import on a real pre-NOT-02 database, and ⌘⇧E / ⌘⇧S against macOS menu shortcuts. | Before the next release |
| DEV-02 | Semantic search from the CLI was tested against a stand-in llama-helper only: check `deino search --json` reports `"mode": "hybrid"` on a Mac with the embedding model downloaded and the bundled `llama-helper` (first search loads the model, about a second), and that no `llama-helper` outlives `deino` or a closed Claude session. | Owner / before the next release |
| DEV-02 | Write tools (`create_action_item`, `update_action_item`, `update_meeting`, private notes for upcoming events, `start_recording`/`stop_recording`) and `ask_deino`, through the running app so writes pass its single writer and `DbChanged` (an HTTP or socket transport into the app), each switchable in Settings → Developer and off by default (D12, D13). The stdio server stays read-only. | DEV-02 phase 2 / DEV-03 |
| DEV-02 | End-to-end Claude Code QA was partial: `claude mcp add` (project scope, sample database) connected and Claude Code 2.1.278 listed all ten `mcp__deino__*` tools, but its OAuth session had expired, so `claude -p "list my last 3 meetings and their action items"` could not run. Run it with a signed-in `claude` (docs/setup/claude-code.md). | Owner |
| DEV-02 | Not verified in a bundled app: `pnpm tauri:build` bundles `Contents/MacOS/cli` (staged by `scripts/stage-cli.sh --release`), signs it with the app, and Install CLI from the DMG links it; `release-macos.yml` builds it. | Before the next release |
| DEV-02 | Links disagree: exports write `deino://meetings/<id>` (EXP-01) while the CLI, MCP tools and DEV-05's card use `deino://meeting/<id>?t=<ms>`; register one shape (and accept the other) when the scheme lands. | DEV-05 |
| DEV-02 | `list_calendar_events` lists every stored event, including calendars hidden in Settings → Calendar (the CLI does not read `settings.json`), and does not name the meeting recorded for an event; `read_meeting` has no insights yet. | Next card touching the CLI / AI-06 |
| DEV-02 | `deino mcp` opens the database once per session and checks the schema then; if the app migrates the database during a long Claude session, re-check (`PRAGMA schema_version`) and reopen or report it. `list_meetings` filters by local day while scanning newest first (a few queries per listed meeting); add one filtered SQL list if libraries grow past ~10k meetings. | Next card touching the CLI |
| AI-05 | Run the live eval with a real key and record the score: `DEINO_EVAL=1 ANTHROPIC_API_KEY=… pnpm eval extract.action_items`. It requires precision ≥ 0.8 and recall ≥ 0.5 over the five fixtures; offline, only the contract, smoke and distractor trials run (against `MockProvider`). | Owner |
| AI-05 | Clicking an item's meeting opens `/meetings/$id?tab=transcript&t=<ms>`; the transcript view scrolls to and marks that moment (TRN-01), and the player seeks there (TRN-03). | ✅ TRN-01 / ✅ TRN-03 |
| AI-05 | Enhanced notes render action items as task blocks: use `useActionItemTodos(meetingId)` / `todoOf` (`features/action-items/notes-sync.ts`) and `useUpdateActionItem` so ticking a block completes the item; the Notes tab's `MeetingActionItems` block can then go. | NOT-02 |
| AI-05 | Place `TodayDigest` (`features/action-items`, the "Due today" list and count) on Home; reuse `ActionItemService::today` and `LocalZone`. | ORG-02 |
| AI-05 | Listen to `plugin:action-items:action-item-changed` (`ActionItemChanged { kind, ids }`) to sync tasks out (INT-03) and to trigger automations (AUT-01); `action_items.external_refs_json` is still untyped. | INT-03 / AUT-01 |
| AI-05 | The 09:00 reminder fires once per day per app run, from 09:00 until noon; a restart inside that window reminds twice. Persist the last reminded day if it matters. | Later cleanup |
| AI-05 | The extraction reads the user's notes as Markdown through NOT-01's `NotesRepo` (clipped to 8,000 characters) and sends the whole numbered transcript; long meetings on the built-in local model may exceed its context. Chunk or summarise first when the route is local. | NOT-02 / AI-01 |
| AI-05 | dev:mock's filter options list people and companies but no tags or folders (`mock-action-item-filter.ts`), and the tag and folder filters match nothing there; read them from `organizer.ts`. | Next card touching the mock |
| AI-05 | ⌘K "New action item" navigates to `/action-items?add=true`; when the page is already open with `add` set, the quick-add field is not refocused. | Next card touching the palette |
| CAP-04 | Embed `VoiceProfiles` (`features/speakers`, a standalone section today shown in Settings → People) on the person page; merging people must move their `voiceprints` and `speaker_suggestions` to `merged_into` (identification already follows `merged_into` when naming); a name typed in the "Who is …?" picker creates a minimal person (name only) that the People pages should let the user complete. Merges move `voiceprints` (keeping the best five) and `speaker_suggestions`, and the person page edits every field; the person page's voice slot (`ENTITY_SLOTS.voice`) shows a summary and links to Settings → People, so embedding the full `VoiceProfiles` list for one person is still open. | ✅ ORG-04 (merge, edit) / CAP-04 (per-person `VoiceProfiles`) |
| CAP-04 | ~~Open `AssignSpeaker` (`features/speakers`) from the transcript's speaker chips with the segment's `segmentId`; show speaker names, colours and the avatar (`SpeakerAvatar`, `displayName`) in the transcript bubbles.~~ Done: each turn's chip opens "Who is …?" for its first line ("This segment" / "All segments"), and lines are named and coloured from `speakers\|participants`. | ✅ TRN-01 |
| CAP-04 | The "Regenerate notes" notice re-runs `NOTES_STAGE` (`features/pipeline/types.ts`, `legacy_summary` today); point it at `enhance_notes`, feed `enhance_notes` the transcript with names through `deino_core::speakers::ParticipantReader` (as `legacy_summary` now does), and offer the same prompt in the Notes tab. NOT-02 (rebased onto CAP-04): `NOTES_STAGE` is `enhance_notes`, and its transcript already names each line by the speaker's person (`MaterialSources` over `SpeakerRepo` and people), so a rename reaches the notes on regeneration. Open: `EnhanceBasis` does not count speaker renames, so the Notes tab's regenerate banner does not offer it; the rerun skips (keeps the notes) when the user edited the newest version. | ✅ NOT-02 (banner: next card touching enhanced notes) |
| CAP-04 | ~~Renames reach insights only once insights exist: read speaker names through `ParticipantReader` when building insight inputs.~~ Insight prompts name speakers through NOT-02's `MaterialSources`, and stored answers are read back with the cited segment's current speaker (`SpeakerNames`), so a rename shows on every answer without a re-run. | ✅ AI-06 |
| CAP-04 | Calibrate `Settings::speakers.thresholds` (0.72 / 0.77 / margin 0.08 / suggest 0.55, measured on synthetic voices) on real dogfood meetings; consider enrolling from re-embedded clean spans (`voiceprints::spans`, the spike's 3 longest overlap-free spans) instead of the diarization centroid. The real-model check is `crates/deino-diarize/tests/voice_profiles.rs` (ignored; anarlog's model files work, `DEINO_DIARIZE_SEGMENTATION` / `DEINO_DIARIZE_EMBEDDING`). | A tuning pass |
| CAP-04 | Map live speaker names onto the post-meeting speakers once live labels exist (the CAP-02 row above): a rename made during the meeting should survive `diarize` replacing the speakers. | CAP-02 / CAP-10 |
| CAP-04 | dev:mock's name picker treats a meeting's speakers' people as its attendees (`mock-speakers.ts`), not `meeting_attendees`; read the fixtures' attendees once the people mock covers them. | ORG-04 |
| INT-00 | Per-provider auth decisions (`docs/research/integrations-oauth.md`): **loopback + PKCE, public** for Google (`127.0.0.1`, any port, Desktop secret sent in the body: `ClientAuth::SecretInBody`), Microsoft (`localhost`, port ignored, no secret) and Linear (fixed `localhost` port the user registered); **user's own app on a fixed port** for Salesforce (External Client App with the secret off); **custom scheme + PKCE** preferred for Slack (user scopes only; fixed-port localhost works meanwhile) and Zoho (fallback: the user's own Self Client); **token paste** for Notion (PAT or internal token), Asana, HubSpot (Service Key), Attio, Todoist, Jira (email + API token, `TokenScheme::Basic`), monday.com, ClickUp (bare `pk_` key), Trello (later a fragment-relay page) and Affinity. Classifiers: Linear `400 RATELIMITED`, Salesforce `403 REQUEST_LIMIT_EXCEEDED`, Zoho `TOO_MANY_REQUESTS`, ClickUp's epoch `X-RateLimit-Reset`. | INT-01..07 |
| INT-00 | Slack's PKCE answer puts the user token under `authed_user`: add a token-response mapper to `OAuthSpec` (or an `Integration` hook) before INT-01. A custom-scheme redirect (`deino://oauth/callback`) needs DEV-05's URL handler; then add `RedirectSpec` a scheme variant. Trello's token arrives in the URL fragment: a loopback page that posts it back. | INT-01 / DEV-05 / INT-03 |
| CAP-02 | ~~The `deino-recording` `live_fixture` tests run on the wall clock and flake under heavy load.~~ CAP-10 paces them by sample count (`tests/support/paced.rs`) and makes the holdback wait for the Them lane's transcript watermark; their nextest retry is gone. `capture_modes` (TRN-05, CAP-08) still records on `FastClock` with 4 threads and one retry: move it onto `rig_on(paced(..))` and drop the override. The gate's Playwright retry is unrelated. | ✅ CAP-10 / Test infra (`capture_modes`) |
| INT-00 | The card's generic "Share to…" menu on the meeting page and the OAuth app form (client ID, secret, port; `set_app`/`remove_app` exist) are not built: no shipped provider needs them under D12. The share menu calls `integrations|targets` and `integrations|deliver` with `share:<meeting>:<connection>:<target>` keys. | First provider card |
| INT-00 | AUT-01 owns the automation `Step` trait; an integration step wraps `IntegrationService::deliver` with a key per run and step and maps `IntegrationError::is_retryable` to `StageError::Retryable`, so the FND-08 runner retries and the key keeps the write single. | AUT-01 |
| INT-00 | Calendar accounts keep `calendar_accounts` rows with refresh tokens under `SecretName` (one per provider). Moving them onto `integration_connections` and `CredentialVault` (several accounts per provider, health and last error on the Integrations page) needs `IntegrationProvider`s for the calendars and a data migration. | Later, if asked |
| INT-00 | The Keychain halves of the `CredentialStore` contract are `#[ignore]`d (they touch the login keychain): run `cargo test -p deino-secrets -- --ignored` on a Mac. The OAuth flow is tested end to end against a local mock authorization server only; a real sign-in waits for the first provider. `integration_deliveries` grows without bound: add it to retention. | Owner / PRV-01 |
| ORG-08 | ~~Summaries are indexed from the legacy `summary_processes`.~~ The newest version of each meeting's enhanced notes (`enhanced_notes.markdown`) is indexed by trigger as kind `summary` and embedded by section; older versions are not searchable. | ✅ ORG-08 (after NOT-02) |
| ORG-08 | Choosing a person or company in ⌘K or on the results page turns it into a filter chip (there are no person or company pages yet); open their pages once ORG-04 ships them, keeping "filter by" as a secondary action. Hits now open the pages; filtering stays available through `@` filters, but there is no "filter by" action on the hit itself. | ✅ ORG-04 (pages) / ORG-08 (secondary action) |
| ORG-08 | ~~Opening a moment at `?t=` is implemented in the legacy transcript (`VirtualizedTranscriptView`).~~ The transcript view opens `?t=` (and `?segment=`) itself and keeps `aria-current="location"` on the linked line; `search.spec.ts` checks that. The player seeks there too (TRN-03). | ✅ TRN-01 / ✅ TRN-03 |
| ORG-08 | ⌘K still has ORG-05/06's own Folders and Tags providers next to search; the `@` picker covers tags and folders as filters. Decide whether folders and tags stay separate groups or become search sections. | Next card touching the palette |
| ORG-08 | The search bench (`search::bench_tests`, 2,000 meetings, `#[ignore]`) and the semantic eval (`llm::retrieval_eval`, needs the model and a built llama-helper) are run by hand; numbers are in ADR 0032. | Owner |
| AI-01 | Embeddings are refreshed by the `index` stage and the backfill only: notes edited after a meeting is processed are searchable by keyword at once but by meaning only after "Index past meetings". Re-embed a meeting a short while after its notes or transcript change (a debounced `DbChanged`-driven job). | AI-02 |
| AI-01 | For natural-language questions the fused ranking can trail the semantic one (OR-matched keywords add noise; eval: hybrid 10/10 top 3, semantic alone 9/10 top 3 but more often first). Tune for chat: weight the lists, or aggregate per meeting, and add more eval questions. | AI-02 |
| AI-01 | The in-memory vector index keeps the vectors of a hard-deleted meeting until the next launch (search hydration drops them, so they never show); remove them on `DbChanged` for `meetings` if memory matters. | Later cleanup |
| AI-01 | The `deino` CLI/MCP gets semantic hits only if it passes `Retriever::open` an `Embedder`: implement one over llama-helper's `embed` request (the model file is `<app data>/models/embedding/nomic-embed-text-v1.5.Q8_0.gguf`, the query prefix `semantic::QUERY_PREFIX`) or over a running app; without it the CLI answers by keyword (FTS5) alone. | DEV-02 / DEV-04 |
| AI-01 | ~~Offer the embedding model in onboarding next to the transcription model.~~ The AI setup step downloads both. | ✅ FND-11 |
| TRN-01 | Bubble actions are registered hidden in `features/transcript/actions.ts`: ~~Play from here, Delete~~ (done, with Edit; the context now carries the query client and whether the recording is playable), Clip and Comment by replacing each entry's `state` and `run` (`rare: true` folds them into ⋯ first). | ✅ TRN-02 / ✅ TRN-03 / TRN-04 / TRN-06 |
| TRN-01 | The toolbar sits beside the bubble on the inner side (a quarter of the column is free there); with all five actions it is about 170 px, wider than the record screen's 380 px panel allows. Collapse the rarer actions into a ⋯ menu when the space runs out. Done: `model/toolbar.ts` fits the buttons to the list's width and folds `rare` actions into ⋯ first. | ✅ TRN-02 |
| TRN-01 | "Who is …?" opens from a turn's chip for the turn's first line; reassigning a later line of a long turn needs a per-line entry (a "Who said this?" bubble action opening `AssignSpeaker` with that line's `segmentId`). Not done in TRN-02 (its scope is text edits): add it as a `rare` action. | TRN-06 / next card touching speakers |
| TRN-01 | ~~Highlight the line being played and words from `words_json`; follow playback.~~ Done: the playing line in `--highlight`, its word a shade deeper, followed while the previous line was on screen (`use-playback-follow.ts`). | ✅ TRN-03 |
| TRN-01 | The inherited session still loads a page of legacy transcript rows just to read the meeting's folder path for the re-transcribe and folder tools; port those tools (and `open_meeting_folder`) and delete `features/meeting/legacy/`. | CAP-08 / CAP-11 |
| TRN-01 | dev:mock's recorder saves its lines without speakers (the view names them You and Them by channel); the Rust recorder stores its Me and Them speakers. Seed them in `mock-speaker-store.ts` if a demo needs renames during a mock recording. | Next card touching dev:mock |
| TRN-01 | `ui.md` offers a solid accent "Me" bubble (Granola) as a setting; the soft tint ships. Add the switch only if the owner wants it. | Owner |
| TRN-01 | The live line's caret is removed when the line is saved; if the caret sat at a wrap boundary the last word can reflow by one line. Reserve the caret's width if this shows in practice. | Later polish |
| ORG-03 | The sidebar's live recording row (`features/shell/sidebar/record-block.tsx`) shows `RecordingSnapshot::title`. Auto-link at start updates it, but a mid-recording re-link from the picker renames only the meeting row. Show the meeting row's title there (`useMeeting(snapshot.meetingId)`), or add `RecordingService::retitle`. | CAP-10 |
| ORG-03 | Linking writes the event's invitees as `meeting_attendees` (organizer / invitee, RSVP); the header's attendee chips and hover cards that show them are ORG-04's. `EventAttendance` adds people through `AttendeeDirectory::person_for`; move it onto ORG-04's entity upsert when that lands. Done: `person_for` goes through `EntityDirectory` (upsert, or `person_with_email` when linking must not add anyone), and the header shows the attendees with hover cards. | ✅ ORG-04 |
| ORG-03 | Opening an upcoming event's note: link it with `EventLinkService::link` (title, series, attendees) rather than `MeetingLifecycleRepo::link`. "Review last meeting" can use `RelatedService` (series, or title and people). | ORG-02 |
| ORG-03 | Audio import's "link to an event": `plugin:calendar|link_event` works on any meeting; reuse `MeetingEventChip` in the import flow. | CAP-11 |
| ORG-03 | The picker suggests the day before the meeting up to an hour after (visible calendars only); there is no search over older events, and suggestions are empty while calendar sync is off. | Later, if asked |
| ORG-03 | Detection's prompt now names an event under way or starting within 15 minutes (it was 15 minutes after the start or 5 before), the same rule as the recording's auto-link. Manual QA on a real Mac: start a recording a few minutes before a calendar event and check the header chip, the attendees and the title; re-link mid-recording; unlink; open a recurring meeting and follow "Previous". | Owner |
| CAP-10 | `RecordingSnapshot::calendar_event_id` is set for recordings started for an event (a detected call, the menu bar's Record, a split). When ORG-03's `LinkOnStart` auto-links a manual start, the snapshot keeps `None`: the menu bar then shows no "Xm left" and auto-stop's calendar rule finds the event by the nearest-event window instead. Let `StartObserver::started` return the linked event, or add `RecordingService::relink` (also for a mid-recording re-link). | ORG-03 / CAP-10 |
| CAP-10 | Manual QA on a real Mac (the agent could not drive a call): unplug the microphone and kill an STT worker while recording (warning within 10 s); type in Zoom or a browser while hovering and clicking the pill (focus never leaves); drag and drop the pill, relaunch (position kept); switch microphone and call audio mid-recording; the menu bar's countdown, "Xm left", Join, Record and Open. | Owner |
| CAP-07 | A split stops this meeting and starts the next one, so about a second of audio between them is in neither. A seamless split would cut both tracks at one timeline position and hand the running capture to a new meeting. | Later, if it matters |
| CAP-07 | Manual QA: one real call that ends (microphone released → "Ending…" → stop), Keep recording, sleep with the lid, and back-to-back events with different invitees. | Owner |
| CAP-10 | The panel shows the first open issue only; with several (no call audio and disk low), the rest are in the sidebar and notifications. A tooltip listing all of them would help. | Later polish |
| ORG-04 | The person and company pages carry a disabled "Ask about …" bar with a WIP badge (D13) in `ENTITY_SLOTS.ask`; fill the slot with the scoped chat. | AI-02 |
| ORG-04 | ⌘J shows an "Ask Deino is coming soon" toast (D13); `ChatPlaceholder` and the layout store's `chatOpen` are unused until chat returns. The sidebar has no "Ask Deino" entry, so no WIP badge was needed there. | AI-02 |
| ORG-04 | People and companies cannot be deleted (only merged); deleting one must decide what happens to its aliases (`merged_into`) and links. | Open |
| ORG-04 | ~~Merging two meetings must move attendance through `meeting_attendees` with the stronger role, as the person merge does (`merge_repo.rs::move_attendance`).~~ Resolved by ORG-07: the merge unites attendance with the stronger role, like the person merge. | ✅ ORG-07 |
| ORG-04 | Enhanced notes keep the absorbed person's label in mention chips after a merge (the ID moves); regenerating refreshes it. | Open |
| ORG-04 | The person and company pages could show external records (CRM contacts and companies) beside the AI briefing. | INT-04 |
| ORG-04 | dev:mock answers `refresh_summary` with `no_model`; a scripted briefing would let the e2e test cover the refresh. | Open |
| TRN-05 | Show corrections in the transcript itself: underline corrected words and offer Undo per segment (the `vocabulary` plugin's `corrections`, and `undo` with a `segmentId`); today the list and Undo live in the meeting's Details panel. | TRN-01 |
| TRN-05 | A live correction is undoable only while its segment keeps the corrected text: diarization's split of a live-corrected segment loses that undo (the stage's own corrections run after the split and keep theirs). Keep per-word originals in `words_json` if this matters. | TRN-02 |
| TRN-05 | One-word names are fuzzy-matched only when the engine capitalised them (≥ 5 letters, same first letter, ≤ 2 letters apart, JW ≥ 0.95); a misheard first name at the start of a sentence that is also a common word is left alone. Revisit with field data. When transcribe-cpp gains Parakeet biasing (`TRANSCRIBE_FEATURE_VOCABULARY`), pass the terms through `RunOptions::vocabulary` too. | Later |
| CAP-08 | The record screen shows no live text with live transcription off; say "The transcript is written after the meeting" there (the recorder's `RecordingSnapshot` could carry `transcribing_live`). Auto-stop on silence has no live VAD in record-only mode (no live pipeline runs); give it a VAD-only path if it needs one. | CAP-10 / CAP-07 |
| CAP-08 | `final_transcription` reports no progress within the stage and has no Cancel in the status strip; a long meeting transcribes for a minute or more. Emit per-utterance progress through `StageCtx.events` and add the cancel command FND-08 lists. | ORG-07 / FND-08 |
| CAP-08 | The high-accuracy re-pass uses the configured engine's batch run (Parakeet Unified with full context, or the "Advanced" engines' text only); a different offline model (Parakeet TDT 0.6B v3 GGUF) for the re-pass is not offered. A manual re-run after notes exist replaces segment IDs, so the notes' citations point at removed segments. | Later / NOT-03 |
| CAP-08 | CPU was measured with mock capture (no device callbacks) on a debug build at load 24: record-only 6.0 %, live on the fake engine 10.1 % of one core. Repeat with real devices and a release build on a quiet machine; in-app per-process accounting (CAP-01 row) is still open. | Owner / CAP-01 |
| FND-11 | **Owner QA on a real Mac with a fresh data folder** (not `com.xenshreyas.deino`'s real data): each prompt (microphone, the system audio probe, the Accessibility prompt, notifications, Calendar), every "Open System Settings" deep link, the 20-second test recording, and the sample meeting (open, delete, import again). dev:mock and the contract tests cannot exercise the macOS prompts. | Owner |
| FND-11 | **Time a fresh install to its first recording** (the card's acceptance: under 3 minutes on a good connection). Not measured on a real Mac yet. Estimate: the Parakeet (731 MB) and embedding (146 MB) downloads take about 70 s at 100 Mbit/s and run while the user moves on; the clicks through six steps take about a minute. | Owner |
| FND-11 | Permission status gaps: macOS cannot report system audio capture or notification permission without asking, so the system audio row shows "Not checked yet" until the user presses Allow (the service then remembers the answer for the session), and the notification plugin reports `granted` on desktop even when alerts are off in System Settings. A tap probe without a prompt, or `UNUserNotificationCenter` settings, would close them. | CAP-10 or the next permissions card |
| FND-11 | The AI step's "On this Mac" choice still embeds the legacy `BuiltInModelManager` with its own styling; port it to the typed `llm` plugin and the setup-row style. | Next card touching Summary models |
| FND-11 | Direct Google and Microsoft calendar connections are hidden in onboarding (they need an app registration; `CalendarSettings connectors={false}`) and stay in Settings → Calendar. Show them in onboarding once a shipped OAuth app removes that step. | ORG-01 follow-up |
| FND-11 | The sample meeting's speakers are labels only (no people or companies are created, so deleting it leaves nothing behind); its notes carry no citations. Citing transcript moments would show off the magnifier. | Open |
| AI-06 | Run the live eval with a real key and record the scores: `DEINO_EVAL=1 ANTHROPIC_API_KEY=… pnpm eval extract.insights` (Decisions recall ≥ 0.6 and ≥ 0.9 attributed over the eight fixtures; a typed scorecard of the sales call). No key was available here; offline, the contract and smoke trials run. | Owner |
| AI-06 | Insight prompts carry the whole transcript, as AI-05's do; long meetings on the built-in local model may exceed its context. Map-reduce over NOT-02's `parts` when the route is local. | Later, with AI-05's row |
| AI-06 | Running insights on past meetings re-runs each meeting's `insights` stage through `PipelineService::rerun`, which moves a `ready` meeting to `processing` while it runs (the status strip shows it). A pipeline "background run" that leaves the meeting's status alone would be calmer. | Next card touching the pipeline |
| AI-06 | AUT-01: reuse `deino_domain::conditions` (add its conditions there: participants, invitee count, duration, calendar, regex) and read `insight_results` for the "generate insight" step; the editor's condition rows (`features/insights/editor/condition-*.tsx`) can move to a shared place then. | AUT-01 |
| AI-06 | The table's cells clip at one line and open per row; a resizable column width and a sticky first column would help wide scorecards. ⌘K has no insight results yet. | Later polish / ORG-08 follow-up |
| AI-06 | dev:mock mirrors the gallery in `mock-insight-templates.ts` (abridged descriptions) and answers from the fixtures (`mock-insight-answers.ts`), not from a model. | — |
| NOT-09 | Action-item extraction (`action_items::extraction::ContextSources`) sends the transcript to `extract.action_items` unredacted; feed its lines through the `TextRedactor` like `MaterialSources`. The same applies to any other route that reads the transcript outside `MaterialSources` (ORG-04's `entity.summary`, AI-02 chat). | AI-05 / ORG-04 / AI-02 |
| NOT-09 | AI-06 insights are redacted only if they gather their input through `MaterialSources::gather`; if they read segments directly, take the `TextRedactor` port. Insight text in exports then needs the same pass in `ExportAssembler`. | AI-06 |
| NOT-09 | Search snippets (`search_fts`, `Retriever`) returned by `deino search` and the MCP `search` tool are not redacted; redact snippets in `deino-core::query::search` with the `ReadContext`'s redactor. | DEV-02 |
| NOT-09 | The LLM check is metered under `tagging` and runs only when that route is the built-in model. Add a `redact.check` route that is always local and not pinnable (like `embed`, with the `llm_calls.route` CHECK migration), so users routing fast tasks to Claude still get the check. | FND-09 |
| NOT-09 | The user's own typed notes reach `notes.enhance` unredacted (the provenance guard renders them verbatim); redacting them means redacting the `Doc`'s text runs before the guard. Exports already redact them. | Later |
| PRV-02 | The app does not use encryption yet: startup `recover` + convert to the `privacy` setting before `DatabaseManager` opens, the key from `DatabaseKeyStore`, the app lock (LocalAuthentication, idle and wake, IPC guard) and Settings → Privacy & security. See `docs/handoff/PRV-02.md`. | PRV-02 (second half) |
| PRV-02 | The Keychain halves of the `DatabaseKeyStore` contract are `#[ignore]`d (login keychain): run `cargo test -p deino-secrets -- --ignored` on a Mac. The `deino` CLI reading the app's key item may prompt once per signature; confirm on a signed build. | Owner |
| PRV-02 | SQLCipher's amalgamation is SQLite 3.45.3 (plain bundled was 3.46.0). Bump libsqlite3-sys together with sqlx. | Whoever bumps sqlx |
| CAP-05 | Manual QA on real calls is the owner's (checklist in `crates/deino-speaker-tags/README.md`): ≥ 90 % of segments named right for speakers who talk > 3 s on Zoom, Meet and Teams. The committed fixture dumps were modelled on each app's call window, not captured on a live call; capture real ones first (`fixtures/README.md`), replace the modelled ones, and adjust the adapters' vocabularies (`GRAMMAR`, Meet's `CLASSES`, call-window rules) to what the apps really expose. | Owner |
| CAP-05 | Calibrate on those calls: the active-speaker indicator lags speech (Zoom ~0.5–1 s); if alignment misses turn starts, shift spans earlier by a measured lag in `Timeline`. Measure Deino's and Chrome's CPU with `AXManualAccessibility` on; if high, re-read only the stage subtree found on the first full read instead of the whole window. | Owner, then a CAP-05 follow-up |
| CAP-05 | Speaker tags have no on/off switch of their own: Accessibility gates them together with CAP-06's browser URLs. Add `Settings::speakers.speakerTags` (default on, as Granola) and a switch on the Settings page if the owner wants one. | Owner |
| CAP-05 | The window's participant list (`Reading::participants`) is read but not stored. Store it per meeting to add `speaker_only` attendees and to resolve names of people who never spoke. | ORG-04 follow-up / later |
| CAP-05 | Names reach the transcript after the pipeline's `identify_speakers`; during the recording the Them lane still shows one speaker. The stored spans (saved every 10 s) can name live lines too. | CAP-10 / TRN follow-up |
| CAP-05 | Meet is read in Google Chrome only (`com.google.Chrome`); Chromium browsers (Arc, Edge, Brave, Chrome Beta/Canary) expose the same tree and need only bundle IDs in `MeetAdapter::bundle_ids` plus QA. Zoom and Teams in a browser are not supported (as in Granola). | Later |
| CAP-05 | FND-11's Accessibility step can call `plugin:speaker-tags|open_accessibility_settings` and link to Settings → Speaker tags; CAP-13's consent message through AX can reuse `deino-speaker-tags`' `WindowReader` and adapters to find each app's chat composer. | FND-11 / CAP-13 |
| TRN-03 | Imported meetings have no `audio_*` paths (the import writes only `folder_path`, outside the recordings folder), so they show no player. Import into `<app data>/recordings/<meeting>/` and set `MeetingLifecycleRepo::set_audio`. | CAP-11 |
| TRN-03 | The screen recording plays when `video_path` is inside the recordings folder (the asset scope); CAP-09 must write it there. The video panel is a fixed 320 px frame above the dock; a larger or detachable view waits for CAP-09's design. | CAP-09 |
| TRN-03 | Not verified in a bundled app: WKWebView playing `audio.mp4`/`mic.m4a` through the asset protocol (range requests, seeking) and `plugin:player|download`'s save sheet. dev:mock plays the fixtures' audio in Chrome; there is no mixed fixture file, so the mock offers Me and Them only. | Manual QA before release |
| TRN-03 | Word highlighting needs `words_json` tokens that spell the line (the fixtures' do); Parakeet's live tokens may be sub-words, in which case lines highlight and seek per line only. Check on a real recording and, if needed, join sub-word tokens when saving. | CAP-01 follow-up |
| TRN-03 | Clips (TRN-04) can reuse `PlayerService::media` for the source files and `playerActions` to preview a range. | TRN-04 |
| TRN-02 | A backfill or `index` stage run that read a transcript before an edit and writes after the re-index can put a stale semantic window back until the meeting is indexed again (ADR 0040). Make `SqliteEmbeddingStore::replace` compare the transcript basis (newest `edited_at`, segment count) inside its transaction. | AI-02 / later cleanup |
| TRN-02 | `original_text` is stored but no UI shows or restores it; add "Show original" / "Revert" to an edited line's ⋯ menu. Undo exists for deletes only (the toast). | Later polish |
| TRN-02 | Find and replace across all meetings scans `transcript_segments` (a LIKE pre-filter); fine for a deliberate bulk edit at thousands of meetings, but use the FTS index to pick candidate meetings if it ever shows. | Later, if slow |
| ORG-09 | Estimated cost covers Anthropic's list prices only (`deino_core::stats::pricing`); OpenAI, Groq and OpenRouter models show as unpriced. Add their prices, or have the gateway record a cost per call. | A small usage card |
| ORG-09 | The year in review is written once per year and cannot be regenerated or shared; add "Write again" and copy or export. | ORG-09 follow-up |
| ORG-09 | Each card that adds an LLM route rebuilds `llm_calls` to widen its CHECK (ORG-09 did for `recap`); a lookup table or dropping the CHECK would end the rebuilds. | Next card adding a route |
| ORG-09 | Hovering the heatmap reads a day out, but its 371 cells are not keyboard stops (each has an accessible label instead); arrow-key navigation over the grid would complete it. | FND-10 polish |
| ORG-07 | The player does not read `meeting_merge_parts` yet: a merged meeting plays only its first recording, while the transcript covers every part. Play the parts back to back on the merged timeline. | TRN-02 |
| ORG-07 | Removing a tag from many meetings: `plugin:meetings\|untag_many` exists and is tested, but the bulk bar only adds. | ORG-07 follow-up |
| ORG-07 | ~~"Run automation" on the bulk bar is a disabled WIP button.~~ Wired by AUT-01: it picks an automation and runs it on the selection (`automations|run`). | ✅ AUT-01 |
| ORG-07 | Restoring a meeting whose pipeline the trash cancelled leaves its remaining stages pending; the status strip offers a re-run, but restore could resume them. | FND-08 / CAP-08 |
| ORG-07 | Bulk export uses a single export's file names and default sections, not Settings → Export's name template, and stops at the first meeting that fails (files already written stay). | EXP-02 |
| ORG-07 | dev:mock ignores the "has action items" and internal/external filters, and a trashed meeting's page, action items and search hits stay reachable there (Rust filters them; the contract tests cover it). | FND-04 |
| ORG-07 | The bulk bar is centred on the window, not the content column, so on windows under ~1000 px it overlaps the sidebar's foot. Centre it on the main column once the shell exposes the sidebar's live width. | FND-10 |
| ORG-07 | A merged speaker whose label clashes with one of the target's gets " (2)"; a second clash on the same label fails the merge. | ORG-07 follow-up |
| AUT-01 | The webhook step (DEV-01) and every provider step (INT-01..07) are listed in the step picker as "WIP" (D12). A card adds an `Action` variant, a `Step` for its `ActionKind` and one `.with(..)` in `LocalStepPorts::registry` (or its own registry call in `composition/automations.rs`); integration steps wrap `IntegrationService::deliver` with a `run:<run>:<position>` key and map `is_retryable` to `StepError` so the stage retries. The payload is `AutomationPayload` (`deino.automation` v1). | DEV-01 / INT-01..07 |
| AUT-01 | The ORG-05 folder page's integrations placeholder can link to `/automations?automation=new` with an `in_folder` condition prefilled. | ORG-05 polish |
| AUT-01 | Delete in the builder's ⋯ menu removes the automation and its runs at once; an Undo toast (or a confirm dialog, ui.md "Danger zone") would be safer once a shared undo pattern exists. | Later polish |
| AUT-01 | Manual QA on a Mac: a real script step (payload on stdin, cwd the meeting folder, killed after 60 s with its children), the export step's folder dialog, a notification step, and the tag trigger arming an upcoming meeting. dev:mock finishes every run as done and does not enforce the shell switch. | Owner |
| AUT-01 | The "Generate insight" step reads the meeting through AI-06's `MaterialSources`; it is redacted only if NOT-09's AI-06 row lands. The run log keeps the latest run per automation and meeting, not a history. | AI-06 / later |

**Critical path to "Granola-equivalent daily driver":**
1. FND-01 → FND-02 → FND-05 / FND-07 → FND-08 → CAP-01 / CAP-02 → NOT-01 → NOT-04 → NOT-02 → ORG-01 → ORG-02 → AI-01 → AI-02

**Critical path to "Circleback-equivalent follow-through":**
1. everything above, then AI-05 → AUT-01 → INT-01 / INT-03 → CAP-03 → CAP-04 → TRN-03 → AI-06 → DEV-02

**Files that are hot spots** (cards that touch these must not run at the same time; §7 already schedules around them):
- `lib.rs` (mitigated by FND-02)
- `audio/pipeline.rs` (CAP-02 only, then CAP-03 reads only)
- `meeting-details` page (handed to FND-10 slots)
- the Sidebar (handed to the FND-10 registry)
- `summary/*` (NOT-02 removed the legacy summaries; only `summary_engine/`, the built-in model's sidecar, remains)

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
| D12 | Priorities (owner, 2026-10-11) | **Direct third-party integrations are deprioritized.**<br>• INT-01..07 (Slack, Notion, task tools, CRMs, Docs/Apple Notes, Gmail/Outlook) and AUT-01's provider actions move to the end of the queue.<br>• The UI shows them with a "WIP" badge. INT-00 ships the framework and the Settings → Integrations page; calendar connectors keep working.<br>• **Promoted:** read access for Claude Code. DEV-02 (stdio MCP) and DEV-04 (CLI) ship first as one `deino` binary that reads the DB directly and read-only. This removes their dependency on AI-02, DEV-03 and ORG-07. Write tools come later via the running app. |
| D13 | "Ask Deino" chat (owner, 2026-10-11) | **Deprioritize the full chat agent; don't remove it.**<br>• AI-02 (the cross-meeting agent) and its dependents (AI-03 write actions, AI-07 MCP-client connectors, AI-08 weekly roundup) move to the back with D12, because Claude Code over MCP (DEV-02) covers cross-meeting questions.<br>• Every UI entry point (sidebar, ⌘K, the person/company/folder chat slot) shows a **"WIP"** badge with a disabled state.<br>• **Still built:** AI-01 semantic/hybrid retrieval, exposed through the MCP `search` tool so Claude Code gets the same retrieval quality.<br>• **Later, before the AI-02 backlog:** a lightweight **"Ask about this meeting"** panel (AI-02-lite), which is single-meeting Q&A with transcript citations and saved prompts (AI-04 recipes scoped to one meeting). |
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
