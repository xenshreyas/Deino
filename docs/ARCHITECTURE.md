# Deino architecture

The map of the system as it is today (FND-13). PARITY_PLAN §4.0 is the target
it grows towards; this file says where each part lives now and how to extend
it. **Update it in the same PR** whenever a card changes a process, a crate, a
module boundary, the data flow, the schema or an extension point (§3A).

Decisions behind this layout are in [`adr/`](adr/): Tauri + Rust core (0004),
SQLite + sqlx (0005), crate layout (0006), typed IPC (0007), TanStack Query +
`DbChanged` (0008), the error model (0009), cloud LLM for notes with local
transcription (0010), the code-quality limits (0011), the core schema with
its legacy `transcripts` view (0012), typed settings with Keychain secrets
(0013), the native crash-safe recording format (0014), the design tokens
and app shell (0015), speech-to-text in supervised transcribe-cpp worker
processes (0016), the LLM gateway with task routes (0017), and the
post-meeting pipeline's stage model (0018) and the network egress policy
(0019).

## 1. Processes

```mermaid
flowchart LR
  subgraph App["Deino.app (one Tauri 2 process)"]
    UI["React UI in WKWebView<br/>frontend/src"]
    Core["Rust core<br/>frontend/src-tauri + crates/"]
    UI <-- "IPC: plugin commands, legacy commands, events" --> Core
  end
  Core -- "stdin/stdout JSON lines:<br/>stream, grammar, embed" --> Llama["llama-helper sidecar<br/>(built-in local LLM)"]
  Core -- "framed stdin/stdout, one per lane + batch" --> Workers["STT workers<br/>(deino --transcribe-worker,<br/>transcribe-cpp + Metal)"]
  Core -- "spawn per job" --> FFmpeg["ffmpeg sidecar<br/>(removed by FND-05, D11)"]
  Core --> Files[("app data dir<br/>deino.sqlite, settings.json, models/, templates/<br/>~/Movies/Deino/&lt;meeting&gt;/audio.mp4")]
  Core --> Keychain[("macOS Keychain<br/>API keys (service = bundle id)")]
  Core -. "deino-net only: scoped per service, logged" .-> LLM["LLM provider APIs,<br/>model hosts, integrations"]
```

- Audio capture and voice activity detection run in the app process.
  Transcription with Parakeet Unified EN 0.6B (the default) and Whisper runs
  in worker processes: the app executable re-launched with
  `--transcribe-worker`, one per live lane plus one for batch work, supervised
  by `deino-stt` (ADR 0016), so a ggml abort or GPU hang never takes the app
  down. Parakeet TDT (the "Advanced" ONNX engine) still runs in-process.
  Notes and chat go to the LLM the user chose, or to the local llama-helper
  (ADR 0010), always through the `deino-llm` gateway, by task route
  (ADR 0017).
- There is no Deino server and no telemetry. Network calls go only to the
  services the user allows, through `deino-net` (FND-12, ADR 0019): each
  caller holds a client scoped to one service's hosts, every request is
  checked against the user's switches and offline mode, and every attempt
  lands in the egress log (Settings → Privacy → Network). The webview has no
  network of its own (`connect-src 'self' ipc: http://ipc.localhost asset:`).

## 2. Rust workspace

```mermaid
flowchart BT
  domain["deino-domain<br/>IDs, enums, units, AppError, Clock"]
  db["deino-db<br/>pools, migrations, change notifier"]
  core["deino-core<br/>features: meetings, settings, segments, people, ..."]
  secrets["deino-secrets<br/>SecretStore, Keychain"]
  pipeline["deino-pipeline<br/>stages, registry, job runner"]
  testkit["deino-testkit<br/>(dev-dependency)"]
  stt["deino-stt<br/>transcribe-cpp workers, supervisor,<br/>live sessions (no network)"]
  app["deino (frontend/src-tauri)<br/>Tauri app: plugins, composition, legacy"]
  helper["llama-helper<br/>(separate binary)"]
  audio["deino-audio<br/>track writer, recovery, finalize"]
  llm["deino-llm<br/>gateway, routes, providers, prompts"]
  net["deino-net<br/>the only HTTP client: scoped clients, egress log, downloads"]
  net --> db
  net --> domain
  app --> net
  diarize["deino-diarize<br/>pyannote + WeSpeaker (ONNX, Core ML),<br/>clustering, voiceprints"]
  knf["knf-rs (vendored)<br/>Kaldi filterbank"]
  diarize --> core
  diarize --> audio
  diarize --> knf
  audio --> domain
  llm --> db
  llm --> domain
  core --> llm
  app --> llm
  db --> domain
  core --> db
  core --> domain
  core --> secrets
  secrets --> domain
  pipeline --> core
  pipeline --> db
  app --> pipeline
  app --> core
  app --> db
  app --> secrets
  app --> stt
  app -. "debug builds: dev plugin" .-> testkit
  testkit -.-> db
```

| Crate | Path | Role |
|---|---|---|
| `deino-domain` | `crates/deino-domain` | The shared vocabulary: typed IDs (`MeetingId`, UUIDv7 IDs), the closed sets the schema stores (`MeetingStatus` with its transitions, `Channel`, `AttendeeRole`, `Rsvp`, ...), `Millis`/`Seconds`, `Email`, `PaletteColor`, `AppError`/`ErrorCode`/`ErrorClass`, the `Clock` port. |
| `deino-db` | `crates/deino-db` | `Database` (WAL, one writer, read pool), `MIGRATOR` over `migrations/`, `DataMigration` (Rust steps between two schema migrations), `ChangeNotifier`, `Table`, `test_db()`. |
| `deino-secrets` | `crates/deino-secrets` | `SecretStore` (set, status, delete, and `get` for Rust only), `KeychainSecretStore` over `keyring`, `InMemorySecretStore` (feature `test-support`). |
| `deino-core` | `crates/deino-core` | Features as Tauri-free modules: `models` (the STT model catalog, verified resumable downloads over a `ByteSource` port), `meetings` (list, rename, lifecycle, trash), `settings` (typed `Settings`, `SettingsService`, the atomic `settings.json` store, the legacy-store importers), one module per core-schema aggregate (`segments`, `people`, `companies`, `calendar`, `folders`, `tags`, `action_items`: ports, SQLite repositories, fakes), and `_example` (debug builds). Also `summaries` (`SummaryGenerator` over the gateway) and `settings::LlmAccess` (credentials, endpoints and route choices for the gateway). `diarization` holds the `diarize` stage's rules (CAP-03): `DiarizeService` over the `TrackDiarizer` port, `plan_channels` (Me on the microphone, the system track diarized, the in-person shared-microphone rule) and `attribute` (segments to speakers by overlap, split at speaker changes). |
| `deino-stt` | `crates/deino-stt` | Speech-to-text (CAP-01): the worker process and its protocol, `EngineSupervisor` (deadlines, GPU→CPU fallback, kill-to-cancel), `LiveSession` (continuous feeds, segments, batch fallback, rotation), `EnginePool` (a worker per lane), `StreamPerf`, the benchmark. Depends on no HTTP client. |
| `deino-llm` | `crates/deino-llm` | The LLM gateway (FND-09, ADR 0017): `Gateway` (`stream`, `complete`, `structured::<T>()`, `with_tools`, `embed`), task `Route`s and the `Router` (user choices over the §4.3 defaults), provider adapters (Anthropic, OpenAI-compatible, Ollama, built-in), the shared SSE parser, `LlmError`, usage metering into `llm_calls`, askama prompts and minijinja user templates, `LlmAdmin` (Settings → AI), `MockProvider` (feature `test-support`) and the eval harness (`tests/llm_eval`). Reaches the world only through ports the app wires. |
| `deino-net` | `crates/deino-net` | The only HTTP client (FND-12, ADR 0019): `NetClient` (send, body, JSON and SSE helpers), `ReqwestNetClient`, the `ServiceId` host registry, `Net::scoped` → `ScopedClient` (host checks, redirects hop by hop, the `ServicePolicy` port, offline mode), the egress log (ring, persister, `SqliteEgressLog`), `HttpDownloader` (resume, size cap, stall timeout, SHA-256), `NetworkControls`. `deino-audio` and `deino-stt` never depend on it (`scripts/check-dependency-rules.sh`). |
| `deino-diarize` | `crates/deino-diarize` | Local speaker diarization (CAP-03, ADR 0019), ported from anarlog: `Diarizer` (batched pyannote segmentation on the CPU, WeSpeaker ResNet34-LM embeddings on Core ML, centroid-linkage clustering under calendar bounds), `policy` (parameters by meeting length), `prune` (phantom speakers), `models` (pinned files downloaded through `deino-core`'s verified downloader), `track::OnnxTrackDiarizer` (the core's `TrackDiarizer` port), `voiceprint` (CAP-04), `online` (live clustering for CAP-02), DER `metrics` and the `diarize-bench` harness. No `tauri`, no `deino-net`. |
| `knf-rs` | `crates/third_party/knf-rs` | Vendored knf-rs 0.3.2 (MIT) over kaldi-native-fbank 1.20.1 (Apache-2.0): the filterbank features the speaker-embedding model reads, built with `cc`. |
| `deino-testkit` | `crates/deino-testkit` | The fixture meetings in `fixtures/` as typed, validated values; `FixtureSeeder` (also behind the debug-only `dev` plugin); fixture audio decoding; re-exported fakes for tests (feature `test-support`). |
| `deino-audio` | `crates/deino-audio` | Crash-safe two-track recording (FND-05a, ADR 0014): the `AudioSource` port and `MockAudioSource`, the real-time ring, `TrackTimeline` (the sample index is time), the writer thread and `AVAssetWriter` CMAF encoder, the persister and `recording.json`, `recover_recording`, `finalize_recording`, health events. Not wired into the app yet (FND-05b). |
| `deino-pipeline` | `crates/deino-pipeline` | The post-meeting pipeline (FND-08, ADR 0018): the `PipelineStage` contract, `registry.rs` (the one list of stages; `legacy_summary` at order 50 for now), the durable `jobs`/`job_runs` queue (`SqliteJobRepo`), the runner (one worker per meeting, 2 stages at once, clock-driven retries, cancellation, `TaskTracker`), startup reconciliation, `PipelineService` and `PipelineTrigger` (enqueues meetings entering `processing`). |
| `deino` | `frontend/src-tauri` | The Tauri app: plugins, composition root, legacy modules. Library crate `deino_lib`. |
| `llama-helper` | `llama-helper` | The local LLM sidecar: `generate` (legacy), `stream`, `grammar` (a JSON schema turned into a GBNF grammar) and `embed` requests over JSON lines. |

Rules (PARITY_PLAN §4.0): dependencies point down the diagram only; **no crate
under `crates/` depends on `tauri`**; a feature becomes its own crate only when
it gains heavy native dependencies or passes about 3k lines. Each crate's
`README.md` lists what it owns and its invariants.

## 3. The app crate (`frontend/src-tauri/src`)

| Module | What it is |
|---|---|
| `lib.rs` | The builder only: plugins, state, setup, run loop. |
| `composition.rs` | **The composition root.** Builds services from adapters (SQLite repos, `SystemClock`, the settings file, the Keychain, Tauri event adapters) and `app.manage`s them. `attach_settings` runs first at startup, then `attach_network` (the one `deino_net::Net` and its policy over the settings, `composition/network.rs`); `attach_database` once `deino.sqlite` is open, which also starts persisting the egress log. |
| `paths.rs` | `AppPaths`: every app folder, resolved once from Tauri's resolver and injected (settings file, legacy stores, models, templates). |
| `llm.rs`, `llm/` | The gateway's wiring: builds `Gateway`, `LlmAdmin` and `SummaryGenerator` over the app's adapters: `NetTransport` (`llm/http.rs`: one `deino-net` scoped client per provider) and `SidecarLocalModels` (`llm/local.rs`, the built-in models over llama-helper). |
| `features.rs`, `features/` | New IPC: one inline Tauri plugin per feature (`system`, `meetings`, `settings`, `secrets`, `llm`, `network`, `pipeline`, `db`, and `example` and `dev` in debug builds), each with one `tauri_specta::Builder`. `features/manifest.rs` lists plugins and commands for the ACL; debug-only plugins are in `DEV_FEATURES` and granted by `capabilities/dev/`. |
| `error.rs` | The command boundary: `into_ipc()` logs the error chain and returns `AppError`. |
| `bindings.rs` | Test-only: exports the TypeScript bindings and checks them against the manifest. |
| `legacy_lints.rs` | The one place inherited modules are exempted from the strict lints. |
| `legacy.rs`, `legacy/` | Every inherited command, in one `generate_handler!`; cards port them into plugins. |
| `setup.rs`, `lifecycle.rs`, `state.rs` | Startup steps, window and exit handling, legacy `AppState`. |
| `database/` | Legacy repositories, and `DatabaseManager`, which wraps `deino_db::Database`. |
| `audio/` | Capture (cpal microphone, Core Audio process tap), mixing, VAD, recording, import, retranscription, the transcription worker. |
| `stt.rs`, `stt/` | Parakeet Unified in the app: `SttRuntime` (Tauri state: engine pool, activity, the live lane), the live lane fed by the pipeline (`live.rs`, 48→16 kHz once), its event sink, the batch `TranscriptionProvider`, `NetByteSource` (the model `ByteSource` over `deino-net`'s model-download client), the worker entry and the `--transcribe-file` benchmark. |
| `features/transcription*` | The `transcription` plugin: model list, download, cancel, delete; events `LiveTranscript`, `ModelChanged`, `ModelDownloadProgress`. |
| `parakeet_engine/`, `whisper_engine/` | The "Advanced" engines: Parakeet TDT (ONNX) and Whisper (`.bin` models, now run through `deino-stt` workers), with their legacy model management. Their downloads, and the built-in model's, take a `deino-net` client per call (`composition::model_downloads`); no engine holds one. |
| `summary/` | The legacy summary commands and `summary_processes` lifecycle (`service.rs`, generation through `deino_core::summaries`), templates, and the built-in model manager and sidecar process (`summary_engine/`). |
| `notifications/`, `tray.rs`, `onboarding.rs`, `updater/` | OS integration. Notification switches and onboarding progress are sections of the typed settings. |

## 4. Frontend (`frontend/src`)

| Path | What it is |
|---|---|
| `main.tsx` | Applies the saved theme, creates the query client and the router (`router.tsx`, which passes `{ queryClient }` to every route's context for loaders) and mounts them. |
| `routes/` | TanStack Router file routes, thin: each renders one feature screen. `/design` (dev builds only) shows every token and primitive in both themes. |
| `styles/` | `tokens.css` (the `ui.md` §2 design tokens for light and dark, mapped to Tailwind utilities with `@theme inline`; ADR 0015), `globals.css` (base layer), `blocknote.css` (the summary editor's bridge to the tokens). |
| `features/<feature>/` | `api.ts` (generated commands, `unwrap`ped), `queries.ts` (keys and hooks with `meta.tables`), `hooks/`, `components/`, `copy.ts`, `types.ts`, `index.ts` (the only import path for other code; `search.ts` for URL params). Today: `meetings` (data and the Meetings page), `meeting` (the meeting page and its slots), `pipeline` (the processing-status strip in the meeting header: stage statuses, retry, `PipelineProgress` refresh), `shell` (sidebar, title bar, ⌘K palette, ⌘J chat bar, shortcuts, layout store), `appearance` (theme), `settings` (`useSettings`, key status, `SettingsSync`, which also applies the saved theme; the sectioned screen), `network` (Settings → Privacy → Network: offline mode, services, the egress log), `llm` (Settings → AI: providers with masked keys, the per-route model table with a streamed Test, and the model lists the inherited pickers use), `record`, `developer`, `design-system` (dev only), `_example`. |
| `lib/ipc/` | `generated/` (tauri-specta output, never edited), `result.ts` (`unwrap`, `IpcError`), `query-client.ts`, `events.ts` (`DbChanged` bridge, `useIpcEvent`), `ipc-query-provider.tsx`. |
| `dev/mock-ipc/` | `pnpm dev:mock` only (never in a production bundle): one mock per generated module (`<feature>.ts`, `satisfies typeof Generated.commands`), an in-memory `MockDatabase` over the fixture meetings that emits `DbChanged`, and a small, shrinking legacy `invoke` shim (`legacy*.ts`). |
| `features/developer/` | Settings → Developer (debug builds only, loaded lazily): seed the fixture meetings through the `dev` plugin. |
| `ui/` | Shared primitives on the tokens (shadcn/Radix restyled: button, icon button, input, select, switch, popover, menu, dialog, tooltip with shortcut, tabs, segmented control, chips, kbd, skeleton, empty state, page header, scroll area, toaster, level bars); `icons.ts` is the only icon import; `cn.ts` merges classes. |
| `components/`, `contexts/`, `hooks/`, `services/`, `lib/*`, `types/` | Legacy code, listed in `legacy-files.mjs`. |

## 5. Data flow

### Record → transcript → notes (today)

```mermaid
sequenceDiagram
  participant Mic as Microphone + system audio
  participant Core as Rust core (audio/, STT)
  participant UI as React UI
  participant DB as deino.sqlite
  Mic->>Core: cpal + Core Audio tap frames
  Core->>Core: mix (audio/pipeline.rs); Parakeet Unified: continuous audio to the live lane (stt/live.rs → deino-stt worker); other engines: VAD segments to transcription/worker.rs
  Core-->>UI: plugin:transcription:live-transcript (growing segment, typed)
  Core-->>UI: transcript-update events (closed segments, legacy, untyped)
  UI->>Core: stop: api_save_transcript (legacy)
  Core->>DB: INSERT meetings + transcripts view (writer connection)
  DB-->>Core: commit hook: tables {meetings, transcript_segments}
  Core-->>UI: plugin:db:db-changed
  UI->>Core: plugin:meetings|list (refetch)
  UI->>Core: api_process_transcript (legacy summary)
  Core->>DB: summary_processes rows; UI polls api_get_summary
```

What changes next: FND-05b moves persistence into Rust during the recording:
it creates the meeting through `MeetingLifecycleRepo::create` (`recording`),
appends each batch of finalized segments with `SegmentWriter::append` (one
transaction, one `DbChanged` per batch) and stops through
`MeetingLifecycleService::transition` (`processing`), which starts the
post-meeting pipeline below.

### Post-meeting pipeline (FND-08, ADR 0018)

```mermaid
sequenceDiagram
  participant Life as MeetingLifecycleService
  participant Trig as PipelineTrigger
  participant Pipe as PipelineService
  participant Run as Runner (worker per meeting)
  participant Stage as PipelineStage (registry order)
  participant DB as jobs / job_runs
  participant UI as React UI
  Life->>Trig: status_changed(recording → processing)
  Trig->>Pipe: enqueue(meeting)
  Pipe->>DB: schedule every registered stage (pending)
  Pipe-->>UI: PipelineProgress(pending) per stage
  Pipe->>Run: wake(meeting)
  loop each stage, in order (2 stages at once across meetings)
    Run->>DB: start: pending → running, open a job_run
    Run-->>UI: PipelineProgress(running)
    Run->>Stage: run(ctx, meeting)
    Stage-->>Run: Done | Skipped | Retryable | Permanent | Cancelled
    Run->>DB: settle (retry: pending with not_before = now + backoff)
    Run-->>UI: PipelineProgress(new status)
  end
  Run->>Life: Complete (ready) or Fail (failed)
  UI->>Pipe: plugin:pipeline|rerun(meeting, stage?) from the status strip
```

At startup `PipelineService::start` reconciles the queue before any worker
runs: it deletes jobs of unregistered stages, settles `running` rows a crash
left (back to `pending`, or `failed` once a crash ate the last attempt),
schedules `processing` meetings without jobs and wakes every meeting with
pending work. On exit `lifecycle.rs` calls `shutdown`, which leaves an
interrupted stage pending for the next start. `legacy_summary` (order 50)
runs the legacy summary job (`summary/service.rs`, over
`deino_core::summaries::SummaryGenerator`) through the `Summarizer` port,
adapted in `features/pipeline/legacy_summary.rs`, until NOT-02's
`enhance_notes` replaces it.

### An LLM call (FND-09)

```mermaid
sequenceDiagram
  participant Caller as Feature service (summaries, …)
  participant Gateway as deino_llm::Gateway
  participant Router
  participant Provider as Provider adapter
  participant Net as HttpTransport / sidecar
  participant Usage as llm_calls
  Caller->>Gateway: complete(Route::NotesEnhance, prompt)
  Gateway->>Router: resolve (ai.routes, Anthropic key?)
  Router-->>Gateway: Target { provider, model }
  Gateway->>Provider: stream(CompletionRequest)
  Provider->>Net: HTTPS + SSE, or a llama-helper stream
  Net-->>Provider: wire events
  Provider-->>Gateway: Text / ToolUse / Usage / Stop
  Gateway-->>Caller: Completion (or a typed T, or a tool loop)
  Gateway->>Usage: route, provider, model, tokens, latency, outcome (no content)
```

The legacy summaries (`api_process_transcript`) pin the provider and model
chosen in their UI on the `notes.enhance` route; Settings → AI tests any
route and streams the reply over a per-request `Channel<LlmStreamEvent>`.

### Settings and secrets (FND-06)

```mermaid
sequenceDiagram
  participant UI as React UI (useSettings)
  participant Settings as SettingsService
  participant File as settings.json
  participant Keychain as macOS Keychain
  UI->>Settings: plugin:settings|patch { ui: { theme: "dark" } }
  Settings->>Settings: RFC 7396 merge, parse into Settings, validate
  Settings->>File: temp file, fsync, rename, fsync dir
  Settings-->>UI: plugin:settings:settings-changed (whole document)
  UI->>Keychain: plugin:secrets|set (via the secrets plugin)
  UI->>Keychain: plugin:secrets|status → missing | stored | locked
  Note over UI,Keychain: no command returns a key; LLM clients read it in Rust
```

Settings are one typed, versioned document in `<app_config_dir>/settings.json`
(on macOS the same folder as the data). Every section is `#[serde(default)]`,
so files from older builds load. The UI changes it with merge patches; Rust
changes it with `SettingsService::update`. A key that macOS refuses to hand
over (an ad-hoc build's new signature) reports `locked`, and the settings
screen asks the user to enter it again.

Legacy stores are imported once and removed: the SQLite `settings` and
`transcript_settings` rows by `LegacySqliteImport`, a `DataMigration` that
runs just before the migration that drops both tables (with `secure_delete`,
so no key bytes survive); the tauri-plugin-store JSON files by
`LegacyFileImport` at startup; the UI's localStorage keys by `SettingsSync`.

### Recording files (`deino-audio`, wired in by FND-05b)

```text
capture callback ─▶ RtSink ═ rtrb ═▶ writer thread (per track, polls 10 ms)
                                     TrackTimeline → Backlog (≤ 64 MiB) → AVAssetWriter (CMAF, 1 s)
                                                                          └─▶ audio/<track>.NNN.m4a.part
persister thread, every 5 s: snapshot frames → F_FULLFSYNC → recording.json (atomic)
stop:    pad both tracks to the stop time → finish → rename .part → .m4a → status closed
launch:  find_unclosed → recover_recording (truncate_to_playable, rename, status recovered)
finalize_recording: mic.m4a, system.m4a (passthrough) + audio.mp4 (−6 dB mix) → status finalized
```

`kill -9` keeps every whole one-second segment (≤ 1 s lost); power loss keeps
what the last 5 s sync covered. Health events (`Stalled`, `FramesDropped`,
`DiskLow`, ...) go to a bounded channel for CAP-10's record pill.

### An outgoing request (FND-12)

```mermaid
sequenceDiagram
  participant Caller as LLM provider, model download, integration
  participant Scoped as ScopedClient (one ServiceId)
  participant Policy as ServicePolicy (network settings)
  participant Root as ReqwestNetClient
  participant Log as EgressLog → egress_log
  Caller->>Scoped: send(Request)
  Scoped->>Policy: grant(service): enabled? offline? custom host?
  alt host not registered, service off, or offline
    Scoped->>Log: entry (blocked, 0 bytes)
    Scoped-->>Caller: NetError (permission_denied)
  else allowed
    Scoped->>Root: send (no automatic redirects)
    Root-->>Scoped: status, headers, body stream
    Note over Scoped: a 3xx is followed here: each hop re-checked, credentials dropped across origins
    Scoped-->>Caller: Response (metered body)
    Scoped->>Log: entry when the body ends: status, bytes up/down
  end
  Log-->>Log: persister writes batches; DbChanged {egress_log} refreshes the Network page
```

Model files come through `net.approved_download(ServiceId::HfModels)`,
which works in offline mode because the user starts every download;
`deino_net::HttpDownloader` adds `.part` resume, a size cap, a stall timeout
and SHA-256 for new model managers.

### Diarization (CAP-03, wired by FND-08 as stage `diarize`, order 30)

```text
DiarizeService::run(meeting)
  plan_channels(segments, attendees):  call → system track (attendees − 1), mic = "Me"
                                       in person (≥ 2 expected, ≤ 3 s system speech) → mic track (all)
                                       mixed (legacy) → audio.mp4 (all)
  TrackDiarizer (deino-diarize::OnnxTrackDiarizer):
      ModelStore::ensure (ByteSource, SHA-256) → read_mono(track, 16 kHz) → policy
      → Diarizer (segmentation batch → shared filterbank → embedding batch → clusters → votes)
      → prune → { turns, voices (centroid, speech) }
  SpeakerRepo::replace_speakers   "Speaker 1..N" + centroid, one transaction (keeps "Me")
  attribute(segment, turns)       words by overlap, split at speaker changes
  SegmentWriter::reattribute      one transaction; the first piece keeps the segment's ID
```

### Reads and writes in a feature

`commands.rs` → `Service` (rules, `Clock`) → `Repo` port → `SqliteXRepo`
(`query*!` macros, reads on the read pool, writes in one transaction on the
writer) → commit hook → `DbChanged { tables, seq }` → the UI invalidates every
query whose `meta.tables` includes a changed table. Legacy code writes through
the same writer connection, so its writes refresh the new queries too.

## 6. Database schema

`<app_data_dir>/deino.sqlite`, migrations in `crates/deino-db/migrations/`
(append-only; never edit an applied file). Only `deino-db` opens the file.

```mermaid
erDiagram
  meetings ||--o{ transcript_segments : has
  meetings ||--o{ speakers : has
  speakers |o--o{ transcript_segments : "said"
  people |o--o{ speakers : "is"
  meetings ||--o{ meeting_attendees : has
  people ||--o{ meeting_attendees : attends
  companies |o--o{ people : employs
  people |o--o{ people : "merged_into"
  calendar_accounts ||--o{ calendar_events : syncs
  calendar_events |o--o{ meetings : "recorded as"
  folders |o--o{ folders : "parent_id"
  folders ||--o{ meeting_folders : holds
  meetings ||--o{ meeting_folders : "filed in"
  tags ||--o{ meeting_tags : labels
  meetings ||--o{ meeting_tags : "tagged"
  meetings |o--o{ action_items : yields
  people |o--o{ action_items : "assigned"
  transcript_segments |o--o{ action_items : "evidence"
  meetings ||--o| summary_processes : has
  meetings ||--o| transcript_chunks : has
  meetings ||--o| meeting_notes : has
  meetings ||--o{ jobs : "processed by"
  jobs ||--o{ job_runs : "attempts"
  meetings {
    TEXT id PK "meeting-<uuid> (opaque text)"
    TEXT title
    TEXT status "scheduled|recording|processing|ready|failed"
    TEXT source "live|import|api"
    TEXT started_at
    TEXT ended_at
    INTEGER duration_s "denormalised: audio length"
    TEXT calendar_event_id FK
    TEXT series_id
    TEXT template_id "NOT-04 makes it a reference"
    TEXT deleted_at "trash"
    TEXT audio_path "and mic, system, video paths"
    TEXT created_at
    TEXT updated_at
    TEXT folder_path "recording folder"
  }
  transcript_segments {
    TEXT id PK "UUID"
    TEXT meeting_id FK
    TEXT channel "mic|system|mixed"
    TEXT speaker_id FK "same meeting (trigger)"
    INTEGER start_ms
    INTEGER end_ms
    TEXT text
    TEXT words_json "Vec<WordTiming>"
    INTEGER is_deleted
    TEXT edited_at
  }
  speakers {
    TEXT id PK
    TEXT meeting_id FK
    TEXT label "unique per meeting"
    TEXT person_id FK
    INTEGER is_me
    INTEGER color "palette slot 1-6"
    BLOB centroid "CAP-03: f32 LE, unit length"
    TEXT embedding_model "CAP-03"
    INTEGER speech_ms "CAP-03"
  }
  people {
    TEXT id PK
    TEXT name
    TEXT email UK "lower case"
    TEXT company_id FK
    INTEGER is_me "at most one"
    TEXT merged_into FK
  }
  companies {
    TEXT id PK
    TEXT name
    TEXT domain UK
  }
  meeting_attendees {
    TEXT meeting_id PK
    TEXT person_id PK
    TEXT role "organizer|invitee|speaker_only"
    TEXT rsvp
  }
  calendar_events {
    TEXT id PK
    TEXT account_id FK
    TEXT provider_event_id "unique per account"
    TEXT series_id
    TEXT start_at
    TEXT attendees_json "Vec<EventAttendee>"
    INTEGER record_override
  }
  folders {
    TEXT id PK
    TEXT parent_id FK "no cycles: FolderRepo::move_folder"
    TEXT name "unique among siblings"
  }
  tags {
    TEXT id PK
    TEXT name UK
    INTEGER color
    INTEGER auto_apply
  }
  meeting_tags {
    TEXT meeting_id PK
    TEXT tag_id PK
    TEXT applied_by "user|ai|automation"
  }
  action_items {
    TEXT id PK
    TEXT meeting_id FK "SET NULL: items outlive meetings"
    TEXT title
    TEXT status "open|done, with completed_at"
    TEXT assignee_person_id FK
    TEXT due_at
    TEXT source_segment_id FK
  }
```

The core schema is FND-07's one migration (`20261009060435_fnd07_core_schema.sql`):
every new table is `STRICT`, enums are `CHECK`ed and mirror a `deino-domain`
enum, foreign keys have an explicit `ON DELETE`, and every foreign key and
list order has an index that a repository test asserts with `EXPLAIN QUERY
PLAN`. No table is `WITHOUT ROWID`, because the update hook behind `DbChanged`
does not see those. `calendar_accounts`, `meeting_folders` and the legacy
tables are omitted from the diagram's attributes.

- **Legacy transcripts** (ADR 0012): `transcripts` is now a view over
  `transcript_segments` with `INSTEAD OF` insert and delete triggers, so the
  inherited import, re-transcription, save, search, delete and meeting-page
  code works unchanged; their writes reach `DbChanged` as
  `transcript_segments`. The view goes when FND-05b, CAP-08, CAP-11 and TRN-01
  have moved the last legacy caller onto `SegmentWriter`/`SegmentReader`.
- **Links live in one place:** a meeting points at its calendar event
  (`meetings.calendar_event_id`); events do not point back.
- **Denormalised copies:** `transcript_chunks.meeting_name` mirrors
  `meetings.title` (`SqliteMeetingRepo::rename` updates both in one
  transaction; legacy summary code does the same). `meetings.duration_s`
  caches the recorded length; `MeetingLifecycleRepo::set_timing` is its only
  writer.
- **`llm_calls`** (FND-09): one row per provider call (route, provider,
  model, tokens, latency, outcome, error kind), never prompt or answer
  content; written by `SqliteLlmCallRepo` through a usage channel.
- **Pipeline queue** (FND-08, migration `20261009071429`): `jobs` has one
  row per (meeting, stage), keyed by both, with the stage's status in the
  current run, its attempts, retry time and user-safe message; `job_runs`
  keeps every attempt (an `INTEGER` rowid key: history rows are never
  referenced). Stage IDs come from the code's registry, not a table;
  startup deletes the jobs of stages that are no longer registered.
- **The egress log** (`egress_log`, FND-12): one append-only row per request
  or redirect hop: `at`, `service`, `host` (with a non-default port),
  `method`, `status`, `outcome` (`sent`/`failed`/`blocked`), `bytes_up`,
  `bytes_down`. There is no column for a URL, query, header or body, and a
  `CHECK` refuses a host with `/` or `?`. The INTEGER rowid orders it;
  `SqliteEgressLog::append` keeps the newest 10,000 rows.
- **Settings are not tables:** FND-06 dropped the single-row `settings` and
  `transcript_settings` tables (migration `20261009061225`); settings live in
  `settings.json` and API keys in the Keychain (ADR 0013).

## 7. IPC

- **Feature plugins** (`plugin:<feature>|<command>`): typed with tauri-specta,
  checked by the capability ACL, generated into
  `frontend/src/lib/ipc/generated/<feature>.ts` by
  `cargo test -p deino --lib bindings`. IDs are branded strings
  (`MeetingId`). Commands return `Result<T, AppError>`; the UI's `unwrap()`
  throws `IpcError { code, message }` (ADR 0007, ADR 0009).
- **Events:** `plugin:db:db-changed` (`DbChanged`),
  `plugin:settings:settings-changed` (`SettingsChanged`, the whole settings
  document), `plugin:llm:ollama-pull` (Ollama download progress), `plugin:pipeline:pipeline-progress` (`PipelineProgress { meetingId, stage,
  status }`, every stage status change), `plugin:example:note-added` (debug). Legacy code still emits untyped events (`transcript-update`,
  `recording-stopped`, …).
- **Legacy commands** (`invoke('api_…')`): about 150 app commands, not
  ACL-checked, in `legacy.rs`. Never add to them.

## 8. Extension points

| To add | Edit |
|---|---|
| A feature | Copy `crates/deino-core/src/_example*`, `frontend/src-tauri/src/features/_example*` and `frontend/src/features/_example/`; then one line each in `features/manifest.rs`, `lib.rs` (`.plugin`), `capabilities/default.json`, `features::builders()`, and the service in `composition.rs`. |
| A table | A migration in `crates/deino-db/migrations/` (`<UTC timestamp>_<card>_<desc>.sql`), a `Table` variant in `crates/deino-db/src/table.rs`, then `scripts/sqlx-prepare.sh` and the bindings test. |
| A command | The function in `features/<feature>/commands.rs` and its `collect_commands!`, the manifest line, the bindings test, and its mock in `frontend/src/dev/mock-ipc/<feature>.ts` (typecheck fails until the mock matches). |
| A fixture scenario | A file in `fixtures/meetings/` (see `fixtures/README.md`); a new table group gets a `SeedTarget` in `FixtureSeeder::targets()`. |
| A typed event | A `#[derive(tauri_specta::Event)]` type in `features/<feature>/events.rs`, `collect_events!`, an event port in the core service if the core raises it. |
| A query | `features/<feature>/queries.ts` with `meta: { tables: [...] }`. |
| A setting | A field in its section in `crates/deino-core/src/settings/` (with its default and any rule in `validate`); the bindings test; the default in `frontend/src/features/settings/settings-fixture.ts` and `dev/mock-ipc/settings.ts`. |
| A secret | A `SecretName` variant in `crates/deino-secrets/src/name.rs`; the bindings test. |
| An outside service (an integration, a provider) | A `ServiceId` variant in `crates/deino-domain/src/service.rs`, its hosts in `crates/deino-net/src/registry.rs`, a migration that rebuilds `egress_log` with the new name in its `CHECK`, and a label in `frontend/src/features/network/copy.ts`. The code that calls it receives `net.scoped(ServiceId::…)` from the composition root, never `reqwest`. |
| A Rust data migration | A `deino_db::DataMigration` whose `runs_before` is the schema migration it precedes, passed to `Database::open_with` (`DatabaseManager::new_from_app_handle`). |
| A downloadable model (file) | An entry in `crates/deino-core/src/models/catalog.json` with its pinned revision, exact size and SHA-256. |
| A transcription lane | `deino_stt::Lane` + `EngineSlot::Live(lane)` (one worker each); start a `LiveSession` per lane from `stt/runtime.rs` (CAP-02 adds the microphone and system lanes). |
| A transcription engine family | A `deino_stt::engine::Engine` implementation served by `worker::run` (transcribe-cpp covers Parakeet and Whisper GGUF/`.bin`). |
| A pipeline stage | A `PipelineStage` in `crates/deino-pipeline/src/stages/<stage>.rs` (idempotent; user-safe messages), one line in `registry::stages` (plus a `StagePorts` field and its adapter in `composition.rs` if it needs a new port), and its label in `frontend/src/features/pipeline/copy.ts`. |
| A diarization model or parameter | `crates/deino-diarize/src/models.rs` (pinned URL, size, SHA-256; a new embedding model needs a new `EMBEDDING_MODEL_ID`) or `policy.rs`; re-run `diarize-bench` (`docs/perf/diarization.md`). |
| An LLM route | A `Route` variant (name, tier) in `crates/deino-llm/src/route.rs`, a migration widening the `llm_calls.route` CHECK, its label in `frontend/src/features/llm/copy.ts`; then call `Gateway` with it. |
| An LLM provider | An adapter in `crates/deino-llm/src/providers/` implementing `LlmProvider` (and `ModelCatalog`), a `ProviderId` variant (and the `llm_calls.provider` CHECK), and one registration in `frontend/src-tauri/src/llm.rs`. |
| A built-in prompt | A `prompts/<name>.<role>.md.jinja` template and its struct and snapshot test in `crates/deino-llm/src/prompts.rs`. |
| An LLM eval | A case in `crates/deino-llm/tests/llm_eval/cases.rs`; run with `DEINO_EVAL=1 pnpm eval <route>`. |
| An integration | INT-00: a `Destination` implementation. |
| An MCP tool | DEV-02. |
| A sidebar entry | `features/shell/nav.ts` plus its route file; set `comingSoon` until the screen exists. |
| A meeting tab, right-panel section or player | `features/meeting/slots.ts` (and the `?tab=` enum in `features/meeting/search.ts` for a tab). |
| ⌘K palette results | A provider component in `features/shell/command-palette/`, appended to `providers.ts`. |
| A global shortcut | `features/shell/shortcuts.ts` and its action in `hooks/use-global-shortcuts.ts`. |
| A settings page | `features/settings/sections.tsx` plus the `?section=` enum in `search.ts` and its copy. |
| A colour or other design token | `styles/tokens.css` (both themes; `tokens.test.ts` checks parity and contrast), then the `/design` gallery. |

## 9. Quality gates

`scripts/ci-local.sh` is the gate: `cargo fmt --check`,
`scripts/check-rust-file-size.sh`, `scripts/check-dependency-rules.sh` (Tauri-free
crates; only `deino-net` has an HTTP client; audio and STT have none),
`cargo deny check licenses bans sources`, the frontend's typecheck, lint,
format check, Vitest, Vite build and the Playwright smoke tests
(`pnpm test:e2e` over `pnpm dev:mock`), `scripts/clippy.sh`, nextest plus
doctests, the bindings freshness check and `scripts/sqlx-prepare.sh --check`.
`.github/workflows/ci.yml` runs the same steps by name on Linux for every push
(the macOS job is manual; `docs/setup/self-hosted-runner.md`), and
`release-macos.yml` builds the unsigned DMG by hand. See
[`TESTING.md`](TESTING.md) and [`REVIEW.md`](REVIEW.md).
