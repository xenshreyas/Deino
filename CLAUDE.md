# CLAUDE.md

Guidance for Claude Code (and humans) working in the Deino repository.

## Read this first

**[`docs/PARITY_PLAN.md`](docs/PARITY_PLAN.md) is the spec.** It defines the
product (§1), the current baseline (§2), the target architecture (§4), the gap
matrix (§5) and the task cards (§6). Its engineering rules are **mandatory for
every change**:

| Section | Rules |
|---|---|
| **§3** | Branch per card, migrations, command registration, frontend structure, settings, LLM gateway, pipeline stages, tests, dependencies, platform guards |
| **§3A** | Code quality: 40-line functions, 300-line files, 4 arguments, nesting depth 3; SOLID; zero-tolerance typing; DB design; layering (`commands` → `service` → `repo` → `adapters`); documentation; testing |
| **§3B** | UI direction: calm, minimal, token-based design, light and dark themes |
| **§3C** | macOS audio and system integration rules |

Then read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) (the map of the
system), [`docs/TESTING.md`](docs/TESTING.md) and [`docs/REVIEW.md`](docs/REVIEW.md).
Decisions are recorded in [`docs/adr/`](docs/adr/).

If you implement a card, the card is your scope. Note anything outside it under
"Follow-ups" instead of doing it.

## What Deino is

A private, local-first AI meeting notepad for one person on macOS. Recording,
transcription (always local), storage and search run on the user's Mac. Data
leaves the Mac only to go directly to services the user connects: the LLM
provider chosen for notes and chat, and integrations. There is **no telemetry**,
no Deino server, and automatic updates are disabled (§8 D6).

- **Identity:** product `Deino`, bundle id `com.xenshreyas.deino`, Cargo package
  `deino` (library crate `deino_lib`), npm package `deino`.
- **Platform:** macOS 14.2+ on Apple Silicon is the primary target. Windows and
  Linux must still compile; new OS-specific features may be stubbed elsewhere.
- **English only:** no language pickers, translation or auto-detection.
  Transcription uses `config::TRANSCRIPTION_LANGUAGE` (`"en"`).

## Commands

```bash
# The whole gate, exactly as CI runs it; run it before every merge
scripts/ci-local.sh                  # every step, cheapest first, fails fast; --list for steps
scripts/ci-local.sh rust             # or a group (rust, frontend) or single steps (clippy test)

# Rust (repository root; the toolchain comes from rust-toolchain.toml)
scripts/clippy.sh                    # clippy -D warnings on every target, then typos
cargo fmt --all --check              # format
cargo test --workspace               # or: cargo nextest run --workspace
scripts/check-rust-file-size.sh      # 300-line cap, tests excluded
scripts/sqlx-prepare.sh              # regenerate .sqlx/ after a query or migration change
cargo test -p deino --lib bindings   # regenerate frontend/src/lib/ipc/generated/, then commit it
scripts/check-dependency-rules.sh    # no tauri under crates/; only deino-net has an HTTP client
cargo deny check licenses bans sources   # permissive licences only (advisories: deny.toml)

# Frontend (frontend/)
pnpm install --frozen-lockfile
pnpm dev                             # Vite dev server on http://localhost:3118
pnpm dev:mock                        # the UI in a plain browser on fixture data, http://localhost:3119
pnpm test:e2e                        # Playwright smoke tests over dev:mock (installed Chrome)
pnpm typecheck                       # baseline + strict flags (scripts/typecheck.mjs)
pnpm lint                            # ESLint; errors fail, warnings only in legacy files
pnpm test                            # Vitest (happy-dom, Testing Library)
pnpm format / pnpm format:check      # Prettier, skipping legacy files
pnpm build                           # typecheck + Vite build into dist/
pnpm run tauri:dev                   # full app; GPU features auto-detected
pnpm run tauri:build                 # production bundle

RUST_LOG=deino_lib::audio=debug pnpm run tauri:dev   # verbose audio logs
```

Build prerequisites: the `llama-helper` sidecar must be staged as
`frontend/src-tauri/binaries/llama-helper-<target-triple>` (see the root README),
and the first build downloads a SHA-256-pinned FFmpeg into the same folder.
`scripts/ci-local.sh` stages a self-describing placeholder when the sidecar is
missing (enough to build and test, not to run the local LLM). CI: Linux on
every push, macOS by hand (`docs/setup/self-hosted-runner.md`).
Optional local hooks: `lefthook install` (`lefthook.yml`).

## Architecture

```
┌──────────────────────────── Tauri 2 app ────────────────────────────┐
│  React UI (frontend/src)  ⇄  typed plugins + legacy invoke/events  ⇄ │
│  TanStack Query, DbChanged         Rust: frontend/src-tauri + crates/ │
└───────────────────────┬───────────────────────────┬──────────────────┘
                        │ sidecars                  │ local files
              llama-helper (llama.cpp)      <app_data_dir>/deino.sqlite
              ffmpeg                        <app_data_dir>/models/…
                                            ~/Movies/Deino/<meeting>/audio.mp4
```

### Rust workspace

| Crate | Role |
|---|---|
| `crates/deino-domain` | Typed IDs, schema enums (`MeetingStatus`, `Channel`, ...), `Millis`/`Email`/`PaletteColor`, `AppError`/`ErrorCode`/`ErrorClass`, the `Clock` port |
| `crates/deino-db` | `Database` (WAL, one writer, read pool), `MIGRATOR` + `migrations/`, `ChangeNotifier`, `Table`, `test_db()` |
| `crates/deino-secrets` | `SecretStore`: API keys in the Keychain (set, status, delete; `get` for Rust only) |
| `crates/deino-core` | Features as modules: `meetings`, `settings`, `templates` (note templates, 30 versioned built-ins, auto selection), `profile` (AI context, the Me person, internal vs external), `models` (STT model catalog and verified downloads), the core-schema repositories (`segments`, `people`, `companies`, `calendar`, `folders`, `tags`, `action_items`), `_example` (debug builds) |
| `crates/deino-audio` | Crash-safe two-track recording: `AudioSource`/`MockAudioSource`, rtrb ring, `TrackTimeline`, AVAssetWriter CMAF writer, `recording.json`, recovery, finalize (ADR 0014) |
| `crates/deino-stt` | Speech-to-text: transcribe-cpp worker processes (`deino --transcribe-worker`), `EngineSupervisor`, `LiveSession`, `EnginePool`, `StreamPerf`; no network (ADR 0016) |
| `crates/deino-llm` | The LLM gateway (ADR 0017): `Gateway` (`stream`, `complete`, `structured::<T>()`, `with_tools`, `embed`), task `Route`s and the `Router`, providers (Anthropic, OpenAI-compatible, Ollama, built-in llama-helper), askama/minijinja prompts, `MockProvider`, the `llm_calls` usage log (no content). Every LLM call goes through it. |
| `crates/deino-pipeline` | Post-meeting pipeline: `PipelineStage`, `registry.rs` (one line per stage), durable `jobs`/`job_runs` runner (retries, cancellation, startup reconciliation), `PipelineService`, `PipelineTrigger` (ADR 0018) |
| `crates/deino-net` | The only HTTP client (FND-12, ADR 0019): `Net::scoped(ServiceId)` → `ScopedClient` (registered hosts, user switches, offline mode), the egress log (`egress_log`), `HttpDownloader` (resume, size cap, SHA-256). Audio and STT never depend on it |
| `crates/deino-detect` | Meeting detection (ADR 0021): Core Audio process-object, default-input and CoreMediaIO camera listeners, outermost-`.app` attribution, Accessibility browser URLs, the bundle tables, Muesli's resolver, the `Clock`-driven `DetectionPolicy`, `DetectionService` and the `Detector` thread; no network |
| `crates/deino-diarize` | Local speaker diarization (ADR 0020): anarlog's pyannote port, WeSpeaker on Core ML, `policy`, `prune`, pinned `models` via the `ByteSource` downloader, `OnnxTrackDiarizer`, `voiceprint`, `online`, DER bench; the stage's rules are `deino-core::diarization` |
| `crates/third_party/knf-rs` | Vendored Kaldi filterbank (knf-rs MIT + kaldi-native-fbank Apache-2.0), built with `cc` |
| `crates/deino-testkit` | Fixture meetings (`fixtures/`) as typed values, `FixtureSeeder`, fixture audio; fakes behind `test-support` |
| `frontend/src-tauri` | The Tauri app (`deino_lib`) |
| `llama-helper` | Local LLM sidecar |

**No crate under `crates/` depends on `tauri`.** Each has a README (Role, Owns,
Does Not Own, Invariants); read it before editing the crate.

### The app crate (`frontend/src-tauri/src/`)

- `lib.rs`: the builder only. `setup.rs` (startup), `lifecycle.rs` (window
  and exit).
- `composition.rs`: **the composition root.** Turns adapters (SQLite repos,
  `SystemClock`, the settings file, the Keychain, Tauri event adapters) into
  services and `app.manage`s them. `paths.rs` (`AppPaths`) resolves every app
  folder once.
- `features.rs`, `features/`: one inline plugin per feature (`system`,
  `meetings`, `settings`, `secrets`, `llm`, `network`, `pipeline`, `db`, and `example`/`dev` in debug builds). `features/manifest.rs`
  lists them for the ACL.
- `error.rs`: `into_ipc()`, the command boundary (log the chain, return
  `AppError`).
- `bindings.rs`: the test that writes the TypeScript bindings.
- `legacy_lints.rs` + `legacy_modules!` in `lib.rs`: the inherited modules and
  the lints they are exempt from. `legacy.rs`: every inherited command.
- `database/`: legacy repositories; `DatabaseManager` wraps `deino_db::Database`
  and hands legacy code the writer pool.
- `audio/` (capture via cpal and the Core Audio process tap `capture/core_audio.rs`,
  mixing in `pipeline.rs`, VAD, recording, import), `stt.rs` + `stt/`
  (Parakeet Unified, the default engine: live lane, batch provider, worker
  entry, `--transcribe-file` benchmark), `parakeet_engine/` and
  `whisper_engine/` (the "Advanced" engines), `summary/`, `notifications/`, `tray.rs`, `onboarding.rs`,
  `updater/` (`AUTO_UPDATE_ENABLED = false`).

### Frontend (`frontend/src/`)

Vite + React 18 + TanStack Router + TanStack Query + Tailwind v4
([ADR 0003](docs/adr/0003-vite-tanstack-router.md), [ADR 0008](docs/adr/0008-tanstack-query-and-dbchanged.md)).

```
src/
├── main.tsx                 query client + IpcQueryProvider + router
├── routes/                  file routes, thin: each renders one feature screen
├── features/<feature>/      api.ts  queries.ts  types.ts  copy.ts  index.ts
│                            hooks/  components/  *.test.ts(x)
├── lib/ipc/                 generated/ (never edit)  result.ts  query-client.ts
│                            events.ts  ipc-query-provider.tsx
├── ui/                      shared primitives on the tokens; icons.ts is the only icon import
├── styles/tokens.css        design tokens, light + dark (ADR 0015); /design shows them (dev)
└── components/ contexts/ hooks/ services/ lib/* types/   legacy (legacy-files.mjs)
```

- Other code imports a feature through its `index.ts` only (`search.ts` for a
  route's URL params). ESLint enforces it.
- Navigate with TanStack Router, never string URLs:
  `navigate({ to: '/meetings/$meetingId', params: { meetingId } })`. Search
  params are validated with zod. Commit `routeTree.gen.ts` with route changes;
  test files inside `routes/` need a `-` prefix.
- Summary editor: BlockNote (frozen until NOT-01 moves to TipTap). ProseMirror
  stays single-instance through the `pnpm.overrides` pins.

## How to add a feature: copy `_example`

`_example` is the reference feature, compiled into debug builds only. Copy its
three parts and rename:

1. **Core:** `crates/deino-core/src/_example.rs` + `_example/` →
   `crates/deino-core/src/<feature>.rs` + `<feature>/{model,error,repo,events,service,fake}.rs`.
   Put the migration in `crates/deino-db/migrations/<UTC timestamp>_<card>_<desc>.sql`
   (not next to the feature, which only the example does), add a `Table`
   variant, run `scripts/sqlx-prepare.sh`. New ID types go in
   `crates/deino-domain/src/id.rs`.
2. **Plugin:** `frontend/src-tauri/src/features/_example.rs` + `_example/` →
   `features/<feature>.rs` + `<feature>/{commands,events,contract_tests}.rs`.
   Register it with one line each in `features/manifest.rs`, `lib.rs`
   (`register_feature_plugins`), `capabilities/default.json`
   (`"<feature>:default"`) and `features::builders()`, and build its service in
   `composition.rs`. Then `cargo test -p deino --lib bindings`.
3. **UI:** `frontend/src/features/_example/` → `features/<feature>/`: `api.ts`
   calls the generated commands through `unwrap`, `queries.ts` declares
   `meta.tables`, components split into a container and presentational parts.

```rust
// features/<feature>/commands.rs: private commands, collected here
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![list, rename]
}

/// Every meeting, newest first.
#[tauri::command]
#[specta::specta]
async fn list(meetings: State<'_, Arc<MeetingService>>) -> Result<Vec<Meeting>, AppError> {
    meetings.list().await.into_ipc()
}
```

```ts
// features/<feature>/queries.ts
export const meetingListQuery = () =>
  queryOptions({
    queryKey: meetingKeys.list(),
    queryFn: meetingsApi.list,
    meta: { tables: ['meetings'] }, // DbChanged keeps it fresh
  });
```

Layers (§3A): `commands.rs` parses, calls one service method and maps the
error; `service.rs` holds the rules over ports and `Clock` (no SQL, no Tauri);
`repo.rs` is SQL only (`sqlx::query*!`); fakes live behind `test-support`.

## Strict rules and the legacy strategy

All new code passes every rule with **zero errors and zero warnings**:

- **Rust:** `[workspace.lints]` (root `Cargo.toml`) + `clippy.toml` thresholds
  (40-line functions, cognitive complexity 15, 4 arguments, nesting) and
  architecture bans (`reqwest::Client` outside `deino-net`, `tokio::spawn`,
  `SystemTime::now`/`Instant::now`/`Utc::now` outside `SystemClock`). Every
  `#[allow]` needs `reason = "…"`. No `unwrap`/`expect`/`panic` outside tests.
- **TypeScript:** `strictTypeChecked`, react-hooks, TanStack Query rules,
  sonarjs complexity 15, `max-lines` 300, `max-lines-per-function` 40,
  `max-params` 4, `max-depth` 3, `noUncheckedIndexedAccess`,
  `exactOptionalPropertyTypes`. No `any`, no unchecked `as`.

Inherited code is exempt through **one list per language, which only shrinks**:

| List | Covers |
|---|---|
| `frontend/src-tauri/src/legacy_lints.rs` (`legacy_modules!` in `lib.rs`) | Rust lints for inherited modules |
| `scripts/legacy-rust-files.txt` | Rust files over 300 lines |
| `frontend/legacy-files.mjs` | ESLint (as warnings), strict TS flags, Prettier |

When a card ports a module or file, it moves it out of the list in the same
PR and makes it pass. Never add new code to a list.

## Rules that are easy to break

- **No raw `useEffect`.** Derive state, use a query or mutation, an event
  handler, or `useMountEffect` (`hooks/useMountEffect.ts`) for mount-only sync
  with something outside React. Reset state by remounting with `key`.
- **Never hand-write `invoke()`.** Call generated bindings through `api.ts`.
- **Never edit an applied migration** or `frontend/src/lib/ipc/generated/`.
- **Commands are private functions** collected in `commands.rs`: a `pub`
  `#[tauri::command]` exports a crate-root macro and collides across features.
- **Every write goes through the writer** (`Database::writer()` or legacy
  `DatabaseManager::pool()`), or the UI never hears about it.
- **A `#[macro_export]` inside a legacy module** cannot be imported by
  `crate::` path; use `pub(crate) use name;`.
- **Never log transcript text, notes, titles, prompts or secrets.** Log IDs
  and counts. Use `perf_debug!()`/`perf_trace!()` on hot paths.
- **Audio callbacks must not allocate, lock or log** (§3C).
- **Paths come from Tauri's resolver** (`app.path().app_data_dir()`), never
  `dirs::` or hard-coded folders.
- Rendering anything under `SidebarProvider` in a test needs a
  `QueryClientProvider`.
- **Colours come from the design tokens** (`bg-bg-muted`, `text-fg-muted`,
  `border-border`, `bg-accent`, …), never Tailwind palette classes or hex
  values (ESLint rejects them), so both themes keep working. Sidebar entries,
  meeting tabs and panels, palette results and settings pages are added
  through their registries (`features/shell/nav.ts`,
  `features/meeting/slots.ts`, `command-palette/providers.ts`,
  `features/settings/sections.tsx`).

## Rust ↔ UI communication

```ts
// UI → Rust: generated bindings through a feature's api.ts, read with TanStack Query
const meetings = await unwrap(commands.list());     // plugin:meetings|list
// Rust → UI: typed events
useIpcEvent(events.noteAdded, ({ note }) => { /* … */ });
// DbChanged (plugin:db:db-changed) invalidates queries by meta.tables automatically.
```

Errors cross IPC as `AppError { code, message }` and arrive as `IpcError`
([ADR 0009](docs/adr/0009-error-model.md)). Legacy commands (`invoke('api_…')`)
and untyped events still exist until cards port them; never add new ones
([ADR 0002](docs/adr/0002-inline-tauri-plugins-per-feature.md),
[ADR 0007](docs/adr/0007-typed-ipc-with-tauri-specta.md)). Never add an
`AppManifest`: it would put every legacy command under the ACL at once.

## Patterns worth knowing

- **Shared state:** services in Tauri state as `Arc<Service>`; legacy code uses
  `Arc<RwLock<T>>` and `Arc<AtomicBool>`. No new global statics.
- **Network:** never build an HTTP client. Take a `ScopedClient` for your
  `ServiceId` from the composition root (`net.scoped(..)`); a new service is
  a `ServiceId` variant plus its hosts in `deino_net::registry`. Model
  downloads use `net.approved_download(ServiceId::HfModels)`.
- **Models:** downloaded into `<app_data_dir>/models/` (`parakeet/`, `summary/`,
  Whisper files at the root), pinned by revision and byte size.
- **Settings:** one typed, versioned document (`deino_core::settings::Settings`,
  `settings.json`), read in the UI with `useSettings()` and changed with merge
  patches (`usePatchSettings`); Rust uses `SettingsService::update`. A new
  setting is a field with a default in its section. **API keys live only in
  the Keychain** (`deino-secrets`); the UI can save, check and forget a key,
  never read it (ADR 0013).

## Platform notes

- **macOS:** system audio uses the Core Audio process tap; **no virtual audio
  driver (BlackHole) is needed**. Permissions: microphone and system audio
  capture (`NSAudioCaptureUsageDescription`). GPU: Metal + CoreML.
- **Windows:** WASAPI loopback for system audio; ONNX Runtime DLLs are bundled
  by `src-tauri/build/onnxruntime.rs`.
- **Linux:** ALSA/PulseAudio; build from source.

## Conventions

- Branches: `feat/<CARD-ID>-<slug>`; never push to `main` or to the `upstream` remote.
- Commits: Conventional Commits (`feat(scope): …`, `fix(…)`, `refactor(…)`,
  `build(…)`, `docs(…)`); `lefthook.yml` checks them. PRs use
  `.github/pull_request_template.md`.
- Naming: audio sources are "microphone" and "system", not "input"/"output".
- Dependencies: pinned, permissive licences only (MIT, Apache-2.0, BSD, ISC);
  `cargo deny check licenses` enforces it. Any binary downloaded at build time
  needs a SHA-256 pin.
- Code copied from MIT/Apache projects gets an entry in `THIRD_PARTY_NOTICES.md`.
