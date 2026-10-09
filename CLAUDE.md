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

All app work happens in `frontend/`.

```bash
pnpm install --frozen-lockfile   # install dependencies
pnpm run dev                     # Next.js dev server on http://localhost:3118
pnpm run build                   # static export into out/ (must pass)
pnpm run tauri:dev               # full app; GPU features auto-detected
pnpm run tauri:build             # production bundle
bun test                         # frontend tests (frontend/tests/)

cd src-tauri
cargo build                      # Rust core
cargo test                       # Rust tests
RUST_LOG=deino_lib::audio=debug pnpm run tauri:dev   # verbose audio logs
```

Build prerequisites: the `llama-helper` sidecar must be staged as
`src-tauri/binaries/llama-helper-<target-triple>` (see the root README), and the
first build downloads a SHA-256-pinned FFmpeg into the same folder
(`src-tauri/build/ffmpeg.rs`).

## Architecture

```
┌──────────────────────────── Tauri 2 app ────────────────────────────┐
│  Next.js UI (frontend/src)  ⇄  invoke()/events  ⇄  Rust core          │
│                                                (frontend/src-tauri)  │
│        ↑ React contexts                      ↓ audio, STT, SQLite,   │
│                                                summaries, tray       │
└───────────────────────┬───────────────────────────┬──────────────────┘
                        │ sidecars                  │ local files
              llama-helper (llama.cpp)      <app_data_dir>/deino.sqlite
              ffmpeg                        <app_data_dir>/models/…
                                            ~/Movies/Deino/<meeting>/audio.mp4
```

### Rust core (`frontend/src-tauri/src/`)

- `lib.rs`: the Tauri builder only (plugins, managed state, feature plugins).
  Startup steps are in `setup.rs`, window/exit handling in `lifecycle.rs`.
- `features.rs`, `features/`: new IPC, one inline Tauri plugin per feature (see
  "Command registration" below). `features/manifest.rs` lists them.
- `legacy.rs`, `legacy/`: every inherited command, in one `generate_handler!`.
  Don't add commands there; cards port them into feature plugins.
- `audio/`: capture, mixing, VAD, recording and import.
  - `devices/`: device discovery and per-platform code (`platform/{macos,windows,linux}.rs`).
  - `capture/microphone.rs`: cpal microphone stream.
  - `capture/core_audio.rs`: **macOS system audio through a Core Audio process
    tap** (`AudioHardwareCreateProcessTap` + an aggregate device named
    `deino-audio-tap`, macOS 14.2+). ScreenCaptureKit is a fallback only.
  - `pipeline.rs`: mixes mic and system audio (today they are mixed before
    VAD/STT; CAP-02 separates them) and feeds the VAD → transcription path and
    the recording path.
  - `recording_manager.rs`, `recording_commands.rs`, `recording_saver.rs`:
    recording lifecycle, Tauri commands, and `audio.mp4` output with checkpoints.
  - `transcription/`: the `TranscriptionProvider` trait with Parakeet and
    Whisper providers.
- `parakeet_engine/`, `whisper_engine/`: model management and inference
  (Parakeet over ONNX Runtime, Whisper over whisper-rs with Metal/CoreML on macOS).
- `summary/`: LLM client, chunk → combine → final processing, templates
  (`summary/templates/`, bundled JSON in `src-tauri/templates/`, user templates
  in `<app_data_dir>/templates/`), and `summary_engine/` for the built-in model.
- `database/`: sqlx SQLite (`deino.sqlite`), repositories and setup. Migrations
  live in `src-tauri/migrations/`; never edit an existing one.
- `notifications/`, `tray.rs`, `onboarding.rs`: OS integration.
- `updater/`: the auto-update gate (`AUTO_UPDATE_ENABLED = false`).

### Frontend (`frontend/src/`)

- Routes: `/` (record), `/meeting-details?id=`, `/settings`.
- State lives in React contexts under `contexts/` plus `components/Sidebar/SidebarProvider.tsx`.
- Summary editor: BlockNote (`components/AISummary/`, `components/BlockNoteEditor/`).
- New work follows §3 rule 4: pages in `app/<route>/`, features in
  `features/<feature>/`, typed IPC wrappers in `lib/ipc/<feature>.ts`.

### Rust ↔ UI communication

```typescript
// UI → Rust: feature plugin command (new code), or a legacy app command
const version = await invoke<string>('plugin:system|app_version');
const meetings = await invoke<Meeting[]>('api_get_meetings');

// Rust → UI: events
await listen<TranscriptUpdate>('transcript-update', (event) => { /* … */ });
```

```rust
#[tauri::command]
async fn my_command(arg: String) -> Result<String, String> { /* thin: call a service */ }

app.emit("transcript-update", payload)?;
```

Commands stay thin (§3A): parse input, call one service function, map errors.

### Command registration: inline plugin

Each feature is one inline Tauri plugin ([ADR 0002](docs/adr/0002-inline-tauri-plugins-per-feature.md)).
Plugin commands are always checked against the capability ACL, so registering
a feature takes three one-line edits: `features/manifest.rs` (from which
`build.rs` generates the ACL permissions), `lib.rs` and
`capabilities/default.json`. A guard test fails if the capability is missing.

```rust
// src/features/meetings.rs (+ features/meetings/commands.rs)
pub const NAME: &str = "meetings";
pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new(NAME)
        .invoke_handler(tauri::generate_handler![commands::list, commands::rename])
        .build()
}
// src/features/manifest.rs   (append-only)
//     ("meetings", &["list", "rename"]),
// src/lib.rs                 .plugin(features::meetings::init())
// capabilities/default.json  "meetings:default"
// UI                         invoke('plugin:meetings|list')
```

`features/system.rs` is the smallest working example. FND-13 switches the
plugin builder to tauri-specta for generated TypeScript bindings. Never add an
`AppManifest`: it would put every legacy command under the ACL at once.

## Patterns worth knowing

- **Hot-path logging:** use `perf_debug!()` / `perf_trace!()` (compiled out in
  release builds). Never log transcript text, notes, prompts or secrets; log IDs
  and counts.
- **Shared state:** `Arc<RwLock<T>>` across async tasks, `Arc<AtomicBool>` for
  flags. Audio callbacks must not allocate, lock or log (§3C).
- **Paths:** resolve directories through Tauri's path resolver
  (`app.path().app_data_dir()`), never a hard-coded or `dirs::`-based app folder.
- **Models:** downloaded into `<app_data_dir>/models/` (`parakeet/`, `summary/`,
  Whisper files at the root). Downloads are pinned (HuggingFace revisions,
  exact byte sizes).
- **Settings:** many still live in localStorage or Tauri store files. FND-06
  introduces the typed settings store and Keychain secrets; use it for anything
  new.

## Platform notes

- **macOS:** system audio uses the Core Audio process tap; **no virtual audio
  driver (BlackHole) is needed**. Permissions: microphone and system audio
  capture (`NSAudioCaptureUsageDescription`). GPU: Metal + CoreML.
- **Windows:** WASAPI loopback for system audio; ONNX Runtime DLLs are bundled
  by `src-tauri/build/onnxruntime.rs`.
- **Linux:** ALSA/PulseAudio; build from source.

## Conventions

- Branches: `feat/<CARD-ID>-<slug>`; never push to `main` or to the `upstream` remote.
- Commits: Conventional Commits (`feat(scope): …`, `fix(…)`, `refactor(…)`, `chore(…)`).
- Naming: audio sources are "microphone" and "system", not "input"/"output".
- Dependencies: pinned, permissive licences only (MIT, Apache-2.0, BSD, ISC).
  Any binary downloaded at build time needs a SHA-256 pin.
- Code copied from MIT projects gets an entry in `THIRD_PARTY_NOTICES.md`.
