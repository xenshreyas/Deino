# Meetily v0.4.1 (Deino fork) — Feature & Architecture Audit

Repo: `/Users/xenshreyas/Documents/GitHub/Deino` (HEAD `aeb1d24 "Start Deino from Meetily v0.4.1"`, upstream `Zackriya-Solutions/meetily`).
Method: read CLAUDE.md, README, docs/, then verified against source. Paths below are relative to the repo root unless absolute. All claims marked "verified" were checked in code; README/CLAUDE.md claims that turned out wrong are called out explicitly.

---

## 0. TL;DR

- **Single-process Tauri 2 desktop app** (Rust core + statically exported Next.js 14 UI). The Python/FastAPI `backend/` is **legacy and not required**; a few dead Rust commands still point at `http://localhost:5167`.
- **Strong core:** local live transcription (Parakeet ONNX default, Whisper via whisper-rs), mic + system audio capture on macOS/Windows/Linux, pause/resume, crash recovery, import audio + re-transcribe, template-driven LLM summaries (7 providers incl. an offline built-in llama.cpp sidecar), editable BlockNote summary, regenerate/cancel, summary-language translation, onboarding, auto-update, opt-in PostHog analytics.
- **Missing (vs Granola/Circleback):** no speaker diarization and no me-vs-them labeling (mic and system are **mixed before transcription**), no user notes during meetings (DB table exists, unused), no AI chat, no calendar, no meeting auto-detection (a macOS detector exists in Rust but nothing calls it), no folders/tags/people, no action-item tracking, no file exports (clipboard copy only), no sharing or integrations, no MCP server, no audio playback UI, no transcript editing, keyword-only search (SQL `LIKE`), no mobile.
- **Rebrand surface:** about 410 string hits across about 100 files. The ones that matter are the bundle id `com.meetily.ai`, the updater endpoint (Zackriya GitHub releases), the PostHog key, the Parakeet model mirror on a Zackriya domain, the FFmpeg download URLs (Zackriya GitHub), the `meeting_minutes.sqlite` DB name, `meetily-recordings` folders and the `Meetily` data dirs.

---

## 1. Architecture map

### 1.1 Tech stack
| Layer | Tech | Where |
|---|---|---|
| Shell | Tauri 2.6 (`macos-private-api`, `tray-icon`, `protocol-asset`) | `frontend/src-tauri/Cargo.toml`, `frontend/src-tauri/tauri.conf.json` |
| UI | Next.js 14 (App Router, `output: 'export'` static), React 18, Tailwind, shadcn/Radix, sonner toasts, framer-motion, BlockNote 0.36 (summary editor), @tanstack/react-virtual | `frontend/package.json`, `frontend/next.config.js` |
| Rust core | tokio, sqlx (SQLite), cpal (patched git rev), cidre (macOS CoreAudio), whisper-rs 0.13 (Metal+CoreML on mac), `ort` 2.0-rc10 (ONNX Runtime for Parakeet), silero VAD, nnnoiseless, ebur128, rubato, symphonia (decode), ffmpeg-sidecar, posthog-rs, reqwest | `frontend/src-tauri/Cargo.toml` |
| Local LLM sidecar | `llama-helper` (llama-cpp-2 0.1.146), JSON over stdin/stdout, bundled as `externalBin` | `llama-helper/src/main.rs`, `frontend/src-tauri/src/summary/summary_engine/sidecar.rs` |
| Bundled binaries | `binaries/llama-helper`, `binaries/ffmpeg` (downloaded at build time by `build/ffmpeg.rs`), Windows `onnxruntime.dll` (`build/onnxruntime.rs`) | `frontend/src-tauri/build.rs`, `frontend/src-tauri/build/` |
| Workspace | Root `Cargo.toml` workspace = `frontend/src-tauri` + `llama-helper` | `/Cargo.toml` |
| Package mgr | pnpm (frontend). Tests use `bun:test`. | `frontend/pnpm-lock.yaml`, `frontend/tests/` |

### 1.2 Process model
- **One Tauri process** hosts the webview (static Next export from `frontend/out`) and the Rust core. All features go through Tauri commands and events.
- **Child processes:** `llama-helper` (only when the "Built-in AI" provider is used), `ffmpeg` sidecar (encoding/merging audio chunks, import decode).
- **External optional services:** Ollama (`http://localhost:11434` default, configurable), and cloud LLM APIs (OpenAI, Anthropic, Groq, OpenRouter, custom OpenAI-compatible).
- **Python backend (`backend/`) is legacy.** Its README says so, and nothing in the app needs it. Leftovers in Rust: `api_get_profile`, `api_save_profile`, `api_update_profile`, `test_backend_connection` and `debug_backend_connection` call `APP_SERVER_URL = "http://localhost:5167"` (`frontend/src-tauri/src/api/api.rs:20`, `:233`). The frontend never invokes the profile commands. `SidebarProvider.tsx:121-122` still sets `serverAddress` to `http://localhost:5167` and `http://127.0.0.1:8178/stream` (vestigial). `frontend/src-tauri/config/backend_config.json` is also vestigial. `backend/whisper.cpp` is an empty git submodule (Zackriya fork).
- **Data persistence flow (important):** live `transcript-update` events are held in **React state** (`contexts/TranscriptContext.tsx`) and mirrored to **IndexedDB** (`services/indexedDBService.ts`, DB `MeetilyRecoveryDB`) for crash recovery. The meeting row and transcripts are written to SQLite **only when recording stops**, when the frontend calls `api_save_transcript` (`services/storageService.ts:44`, `hooks/useRecordingStop.ts:256`). Rust also writes `transcripts.json`, `metadata.json` and `audio.mp4` into a per-meeting folder (`audio/recording_saver.rs`, `audio/incremental_saver.rs` with checkpoint chunks).
- Closing the main window hides it to the tray (`lib.rs:600-611`). A single-instance guard is in place.

### 1.3 Directory map
| Path | Contents |
|---|---|
| `frontend/src/app/` | Routes: `/` (`page.tsx`, live recording screen), `/meeting-details?id=` (`meeting-details/page.tsx` + `page-content.tsx`), `/settings` (`settings/page.tsx`), `/notes/[id]` (**hard-coded sample notes, dead demo page**). `_components/` holds the home-page panels. `layout.tsx` holds the provider tree, onboarding gate, drag-drop import and tray-toggle listener. |
| `frontend/src/components/` | `Sidebar/` (meeting list, search, nav, global context), `MeetingDetails/` (split view, transcript and summary panels, retranscribe dialog), `AISummary/` and `BlockNoteEditor/` (summary editor), `onboarding/` (4-step flow), `ImportAudio/`, `TranscriptRecovery/`, `DatabaseImport/` (legacy DB import), model managers (`WhisperModelManager`, `ParakeetModelManager`, `BuiltInModelManager`), `ModelSettingsModal.tsx` (LLM provider config), `TranscriptSettings.tsx`, `RecordingControls.tsx`, `DeviceSelection.tsx`, `About.tsx`, `Analytics*`, `Update*`, `ui/` (shadcn) |
| `frontend/src/contexts/` | `ConfigContext` (model/transcript config, devices, beta flags, auto-summary), `RecordingStateContext` (syncs with Rust recording state), `TranscriptContext`, `OnboardingContext`, `OllamaDownloadContext`, `ImportDialogContext`, `RecordingPostProcessingProvider` |
| `frontend/src/hooks/` | Recording start/stop/sync, transcript recovery, paginated transcripts, `meeting-details/*` (summary generation, templates, copy ops, model config), `useAudioPlayer.ts` (**unused**) |
| `frontend/src/services/` | `storageService` (SQLite via Tauri), `indexedDBService` (recovery), `recordingService`, `transcriptService`, `updateService`, `configService` |
| `frontend/src/lib/` | `analytics.ts`, `builtin-ai.ts`, `whisper.ts`, `parakeet.ts`, summary helpers, `blocknote-markdown.ts` |
| `frontend/src-tauri/src/` | `lib.rs` (entry and command registry), `audio/` (capture, pipeline, VAD, recording, import, retranscription, devices), `whisper_engine/`, `parakeet_engine/`, `summary/` (LLM client, processor, templates, built-in AI sidecar), `database/` (sqlx manager, repositories), `api/` (meeting CRUD and settings commands), `ollama/`, `openai/`, `anthropic/`, `groq/`, `openrouter/` (model listing), `analytics/` (PostHog), `notifications/`, `tray.rs`, `onboarding.rs`, `console_utils/` |
| `frontend/src-tauri/migrations/` | 10 sqlx migrations (see 1.4) |
| `frontend/src-tauri/templates/` | 6 bundled summary template JSONs plus README |
| `llama-helper/` | llama.cpp sidecar binary crate |
| `backend/` | Legacy FastAPI, Docker and whisper-server (unsupported) |
| `scripts/` | `inject_transcript.py` (dev: inject CSV transcript into DB), `generate-update-manifest-github.js`, `test-update-locally.js` |
| `.github/workflows/` | build (macos/windows/linux/devtest/test), release, pr-main-check (version validation only, **no tests run in CI**) |
| `docs/` | `architecture.md` (high level), `BUILDING.md`, `building_in_linux.md`, `GPU_ACCELERATION.md`, screenshots |

**Dead or uncompiled Rust files** (not declared in any `mod`): `src/lib_old_complex.rs`, `src/audio_v2/*` (stub "Phase 3/4" TODOs), `src/audio/stt.rs`, `src/audio/core-old.rs`, `src/audio/recording_saver_old.rs`, `src/audio/recording_commands.rs.backup`, `src/whisper_engine/_stderr_suppressor.rs`, `src/audio/system_audio_types.ts` (a TS file inside the Rust tree). Also committed: `frontend/vs_buildtools.exe` (4.3 MB), `frontend/src-tauri/logs/meeting-minutes.log.2024-12-30`.

### 1.4 Database (SQLite via sqlx)
- File: `<app_data_dir>/meeting_minutes.sqlite`. With identifier `com.meetily.ai` this is `~/Library/Application Support/com.meetily.ai/` on macOS. Defined in `frontend/src-tauri/src/database/manager.rs:55-67`. The legacy `meeting_minutes.db` is auto-imported if present. WAL mode, with defensive WAL/SHM cleanup.
- Migrations: `frontend/src-tauri/migrations/*.sql`. Final schema:

| Table | Columns | Used? |
|---|---|---|
| `meetings` | `id` PK, `title`, `created_at`, `updated_at`, `folder_path` | Yes |
| `transcripts` | `id` PK, `meeting_id` FK, `transcript`, `timestamp`, `summary`, `action_items`, `key_points`, `audio_start_time`, `audio_end_time`, `duration`, `speaker` | Yes for text and timing. `summary`, `action_items` and `key_points` are legacy and unused. **`speaker` is added by a migration but never read or written in Rust or TS.** |
| `summary_processes` | `meeting_id` PK, `status`, `created_at`, `updated_at`, `error`, `result` (JSON with markdown), `start_time`, `end_time`, `chunk_count`, `processing_time`, `metadata`, `result_backup`, `result_backup_timestamp` | Yes. The summary lives in `result`. The backup supports regenerate/cancel. |
| `transcript_chunks` | `meeting_id` PK, `meeting_name`, `transcript_text`, `model`, `model_name`, `chunk_size`, `overlap`, `created_at` | Yes (summary input cache) |
| `settings` | `id`, `provider`, `model`, `whisperModel`, `groqApiKey`, `openaiApiKey`, `anthropicApiKey`, `ollamaApiKey`, `openRouterApiKey`, `ollamaEndpoint`, `customOpenAIConfig` (JSON), `geminiApiKey` | Yes. **API keys are stored in plaintext.** `geminiApiKey` is unused (ConfigContext.tsx:119 says "Gemini omitted for now"). |
| `transcript_settings` | `id`, `provider`, `model`, `whisperApiKey`, `deepgramApiKey`, `elevenLabsApiKey`, `groqApiKey`, `openaiApiKey` | provider and model used. The cloud STT key columns are vestigial. |
| `licensing` | `license_key` PK, `encrypted_key`, `signature_hash`, `activation_date`, `expiry_date`, `soft_expiry_date`, `max_activation_time`, `duration`, `generated_on`, `is_soft_expired`, `grace_period` | **Unused**. A PRO licensing remnant with no Rust code referencing it. |
| `meeting_notes` | `meeting_id` PK/FK, `notes_markdown`, `notes_json`, `created_at`, `updated_at` | **Unused.** Schema only, with no commands or UI. |

- Per-meeting language preferences are stored in the meeting folder's `metadata.json`, not in the DB (`summary/metadata.rs`, `summary/commands.rs:133-215`).
- Repositories: `database/repositories/{meeting,transcript,summary,setting,transcript_chunk}.rs`.

### 1.5 Audio capture per OS
Recording entry: `audio/recording_commands.rs` → `audio/recording_manager.rs` → `audio/stream.rs`, `audio/capture/{microphone,system,core_audio}.rs` → `audio/pipeline.rs`.

| OS | Microphone | System audio | Notes |
|---|---|---|---|
| macOS (min **14.2**, per `.cargo/config.toml`) | cpal | **Default: Core Audio process tap**. Uses `cidre`, `TapDesc::with_mono_global_tap_excluding_processes` and an aggregate device named `meetily-audio-tap` (`audio/capture/core_audio.rs`). Alternate backend "ScreenCaptureKit" goes through cpal (`audio/capture/backend_config.rs`). Selectable via `set_audio_backend`. | Needs Audio Capture permission (NSAudioCaptureUsageDescription, macOS 14.4+) and mic permission. **CLAUDE.md's claim that BlackHole is required is outdated.** The tap captures all system output without a virtual device. |
| Windows | cpal (WASAPI) | WASAPI loopback on output devices (`audio/devices/platform/windows.rs`) | ONNX Runtime DLL bundled and loaded at startup (`lib.rs:478-505`) |
| Linux | cpal (ALSA) | PulseAudio "monitor" sources enumerated through the ALSA host (`audio/devices/platform/linux.rs:17-22`) | Build from source only, no official package |

Pipeline (`audio/pipeline.rs`):
- Each `AudioCapture` applies per-device processing. Mic only: RNNoise suppression, high-pass filter and normalizer (`pipeline.rs:283`).
- `AudioMixerRingBuffer` aligns mic and system into 50 ms windows. `ProfessionalAudioMixer` sums them with clipping prevention.
- **The mixed signal** goes to (a) Silero VAD, then the transcription channel, and (b) the recording saver. Every chunk is tagged `DeviceType::Microphone // Mixed audio` (`pipeline.rs:851-900`). **Channel identity is lost before STT**, so "me vs them" is impossible without changing the pipeline.
- Recording output is `audio.mp4` (AAC via ffmpeg), checkpointed into `.checkpoints/audio_chunk_NNN.mp4` and merged on stop (`incremental_saver.rs`).
- Extras: device hot-swap and disconnect handling (`device_monitor.rs`, events `mic-device-switched`, `mic-unavailable`), Bluetooth playback warning (`playback_monitor.rs`), audio level meters (`simple_level_monitor.rs`), an "audio wake" silence trick (`recording_manager.rs`).

### 1.6 Transcription engines
- Abstraction: `audio/transcription/{engine.rs,provider.rs,whisper_provider.rs,parakeet_provider.rs,worker.rs}`. The `TranscriptionProvider` trait is where new engines plug in. `engine.rs` selects from `transcript_settings.provider`: `"localWhisper"` or `"parakeet"` (default parakeet).
- **Parakeet** (default): `parakeet_engine/` via `ort`. Models: `parakeet-tdt-0.6b-v3-int8` (default, downloaded from **`https://meetily.towardsgeneralintelligence.com/models/parakeet-tdt-0.6b-v3-onnx`**, a Zackriya-hosted mirror) and `parakeet-tdt-0.6b-v2-int8` (HuggingFace istupakov, pinned rev). Defined in `parakeet_engine/parakeet_engine.rs:107-136`.
- **Whisper**: `whisper_engine/` via whisper-rs with GPU autodetect (`acceleration.rs`). Catalog in `config.rs` and `whisper_engine.rs:1077-1088`: tiny, base, small, medium, large-v3, large-v3-turbo and their q5 variants, from HuggingFace ggerganov. Default `large-v3-turbo`. `parallel_processor.rs` provides batch parallel transcription commands.
- Language: global `LANGUAGE_PREFERENCE` (default `"auto-translate"`) in `lib.rs:125`, set via `set_language_preference`.
- **Cloud STT** (Deepgram, ElevenLabs, Groq, OpenAI) appears only as **commented-out UI options** in `components/TranscriptSettings.tsx:126-128` plus DB columns. There is no Rust implementation. Stub.
- Live worker emits `transcript-update` events. The payload has `source: "Audio"` (`transcription/worker.rs:209`), so it carries no speaker information.

### 1.7 LLM summaries
- Providers (`summary/llm_client.rs:215-236`): `OpenAI`, `Claude` (Anthropic Messages API), `Groq`, `Ollama`, `OpenRouter`, `BuiltInAI` (llama-helper sidecar), `CustomOpenAI` (any OpenAI-compatible endpoint with maxTokens, temperature and topP). The UI list is in `components/ModelSettingsModal.tsx:877-883`. Model-list commands: `get_ollama_models`, `get_openai_models`, `get_anthropic_models`, `get_groq_models`, `get_openrouter_models`. Ollama also supports pull and delete.
- Built-in AI models (`summary/summary_engine/models.rs`): Qwen3.5-2B Q4_K_M, Qwen3.5-4B Q4_K_M, Gemma-3-1B Q8_0, Gemma-3-4B Q4_K_M (HuggingFace unsloth and bartowski). RAM-based recommendation. Managed by `model_manager.rs`.
- Pipeline (`summary/processor.rs`, `summary/service.rs`):
  - Token-based chunking (`chunk_text`), then per-chunk summaries (`build_chunk_summary_user_prompt`), then a combine step, then a final report. The final system prompt is built from the template (`build_final_report_system_prompt`).
  - Optional "User Provided Context" (the free-text "custom prompt" textarea under the transcript, `components/MeetingDetails/TranscriptPanel.tsx:100-110`) is appended in `<user_context>`.
  - Reasoning-marker stripping, English normalization, and optional translation to a target summary language (`language_detection.rs` via `whatlang`).
  - Runs as a background task with a cancellation token. The frontend polls `api_get_summary`. A cache fingerprint covers transcript, template and language.
- **Prompts live in Rust** (`summary/processor.rs:113-255`). **Templates** live in `frontend/src-tauri/templates/*.json`. Two are compiled in (`summary/templates/defaults.rs`). The rest are loaded from bundled resources. Users can add custom templates (see Feature inventory).

### 1.8 Settings storage
| Store | What |
|---|---|
| SQLite `settings` / `transcript_settings` | Summary provider, model and API keys (plaintext), Ollama endpoint, custom OpenAI config, transcription provider and model |
| tauri-plugin-store `recording_preferences.json` | save folder (default `~/Movies/meetily-recordings` on macOS, `~/Music/meetily-recordings` on Windows, `~/Documents/meetily-recordings` elsewhere), auto_save, file_format (mp4), preferred mic and system devices, macOS audio backend (`audio/recording_preferences.rs`) |
| store `onboarding-status.json` | onboarding progress (`onboarding.rs`) |
| store `analytics.json` | opt-in flag, user id, platform info, counters |
| store `preferences.json` | notification and other prefs (frontend) |
| store `store.json` | legacy server address (api.rs:209) |
| notifications settings file | `dirs::config_dir()/meetily/...` (`notifications/settings.rs:118`) |
| `localStorage` | `betaFeatures`, `isAutoSummary`, `providerModelMap`, `primaryLanguage`, `showConfidenceIndicator`, `downloading-models`, `transcription_progress`, `meetily.meetingDetails.transcriptPaneRatio` |
| IndexedDB `MeetilyRecoveryDB` | in-progress transcripts for crash recovery |

### 1.9 Tauri commands (registered in `frontend/src-tauri/src/lib.rs:612-833`, about 190 total)
- **Recording:** `start_recording`, `stop_recording`, `is_recording`, `start_recording_with_devices`, `start_recording_with_devices_and_meeting`, `pause_recording`, `resume_recording`, `is_recording_paused`, `get_recording_state`, `get_meeting_folder_path`, `get_transcript_history`, `get_recording_meeting_name`, `get_active_audio_output`, `get_transcription_status` (stub that returns zeros), `read_audio_file`, `save_transcript` (writes an arbitrary file path), `start/stop/is_audio_level_monitoring`, `get_audio_devices`, `trigger_microphone_permission`, `set_language_preference`
- **Recovery:** `recover_audio_from_checkpoints`, `cleanup_checkpoints`, `has_audio_checkpoints`
- **Whisper (12):** `whisper_init`, `whisper_get_available_models`, `whisper_load_model`, `whisper_get_current_model`, `whisper_is_model_loaded`, `whisper_has_available_models`, `whisper_validate_model_ready`, `whisper_transcribe_audio`, `whisper_get_models_directory`, `whisper_download_model`, `whisper_cancel_download`, `whisper_delete_corrupted_model`, plus `open_models_folder`
- **Parakeet (14):** `parakeet_init`, `parakeet_get_available_models`, `parakeet_load_model`, `parakeet_get_current_model`, `parakeet_is_model_loaded`, `parakeet_has_available_models`, `parakeet_validate_model_ready`, `parakeet_transcribe_audio`, `parakeet_get_models_directory`, `parakeet_download_model`, `parakeet_retry_download`, `parakeet_cancel_download`, `parakeet_delete_corrupted_model`, `open_parakeet_models_folder`
- **Parallel whisper (11):** `initialize_parallel_processor`, `start/pause/resume/stop_parallel_processing`, `get_parallel_processing_status`, `get_system_resources`, `check_resource_constraints`, `calculate_optimal_workers`, `prepare_audio_chunks`, `test_parallel_processing_setup`
- **Meetings and settings (`api/api.rs`):** `api_get_meetings`, `api_search_transcripts`, `api_get_meeting`, `api_get_meeting_metadata`, `api_get_meeting_transcripts` (paginated), `api_save_meeting_title`, `api_save_transcript`, `api_delete_meeting`, `open_meeting_folder`, `api_get_model_config`, `api_save_model_config`, `api_get_api_key`, `api_get_transcript_config`, `api_save_transcript_config`, `api_get_transcript_api_key`, `api_save_custom_openai_config`, `api_get_custom_openai_config`, `api_test_custom_openai_connection`, `open_external_url`. Legacy remote: `api_get_profile`, `api_save_profile`, `api_update_profile`, `test_backend_connection`, `debug_backend_connection`.
- **Summary:** `api_process_transcript`, `api_get_summary`, `api_save_meeting_summary`, `api_cancel_summary`, `api_get/save_meeting_summary_language`, `api_get/save_meeting_detected_summary_language`, `api_detect_transcript_summary_language`
- **Templates:** `api_list_templates`, `api_get_template_details`, `api_validate_template`
- **Built-in AI:** `builtin_ai_list_models`, `builtin_ai_get_model_info`, `builtin_ai_download_model`, `builtin_ai_cancel_download`, `builtin_ai_delete_model`, `builtin_ai_is_model_ready`, `builtin_ai_get_available_summary_model`, `builtin_ai_get_recommended_model`
- **LLM model listing:** `get_ollama_models`, `pull_ollama_model`, `delete_ollama_model`, `get_ollama_model_context`, `get_openai_models`, `get_anthropic_models`, `get_groq_models`, `get_openrouter_models`
- **Recording prefs and backend:** `get/set_recording_preferences`, `get_default_recordings_folder_path`, `open_recordings_folder`, `select_recording_folder`, `get_available_audio_backends`, `get_current_audio_backend`, `set_audio_backend`, `get_audio_backend_info`
- **Notifications (14):** `get/set_notification_settings`, `request_notification_permission`, `show_notification`, `show_test_notification`, `is_dnd_active`, `get_system_dnd_status`, `set_manual_dnd`, `set_notification_consent`, `clear_notifications`, `is_notification_system_ready`, `initialize_notification_manager_manual`, `test_notification_with_auto_consent`, `get_notification_stats`
- **System audio and permissions:** `start_system_audio_capture_command`, `list_system_audio_devices_command`, `check_system_audio_permissions_command`, `start_system_audio_monitoring`, `stop_system_audio_monitoring`, `get_system_audio_monitoring_status`, `check/request_screen_recording_permission_command`, `trigger_system_audio_permission_command`, `open_system_settings` (macOS)
- **Database:** `check_first_launch`, `select_legacy_database_path`, `detect_legacy_database`, `check_default_legacy_database`, `check_homebrew_database`, `import_and_initialize_database`, `initialize_fresh_database`, `get_database_directory`, `open_database_folder`
- **Onboarding:** `get_onboarding_status`, `save_onboarding_status_cmd`, `reset_onboarding_status_cmd`, `complete_onboarding`
- **Retranscription and import:** `start/cancel/is_retranscription_in_progress_command`, `select_and_validate_audio_command`, `validate_audio_file_command`, `start_import_audio_command`, `cancel_import_command`, `is_import_in_progress_command`
- **Analytics (25):** `init_analytics`, `disable_analytics`, `track_event`, `identify_user`, `track_*` (meeting, recording, summary, settings, model, custom prompt, analytics toggles), `start/end_analytics_session`, `is_analytics_enabled`, `is_analytics_session_active`
- **Console:** `show_console`, `hide_console`, `toggle_console`
- **Broken calls:** the frontend invokes `api_get_auto_generate_setting` (`components/ModelSettingsModal.tsx:320`) and `builtin_ai_get_models_directory` (`lib/builtin-ai.ts:96`), and **neither is registered**. Both fail at runtime and are caught.
- **Key events (Rust → UI):** `transcript-update`, `transcription-progress`, `transcription-error`, `transcription-warning`, `transcription-queue-complete`, `speech-detected`, `recording-started`, `recording-starting`, `recording-stopped`, `recording-paused`, `recording-resumed`, `recording-error`, `recording-stop-complete`, `mic-device-switched`, `mic-swap-failed`, `mic-unavailable`, `request-recording-toggle` (tray), `system-audio-started`, `system-audio-stopped`, and model download progress events.

### 1.10 Frontend routes and major components
- `/`: live recording screen. `TranscriptPanel` uses `VirtualizedTranscriptView` with a typewriter effect (`useTranscriptStreaming`). `RecordingControls` handles start, pause and stop with a level meter. Recovery dialog. Status overlays.
- `/meeting-details?id=`: `MeetingDetailsSplitView` (resizable transcript and summary panes, tabs). `TranscriptPanel` has copy, open folder, retranscribe and the custom-context textarea. `SummaryPanel` has a template picker, model picker, generate/regenerate/stop, a BlockNote editable summary, and save and copy.
- `/settings`: tabs General, Recording, Transcription models, Summary models, Beta.
- Sidebar: home, settings, search box, a single hard-coded "Meetings" folder with the meeting list, rename and delete, import audio, and start recording. The About dialog opens from the logo.
- `/notes/[id]`: **static demo data** (`app/notes/[id]/page.tsx`). Not real.

### 1.11 Build and test commands
- Dev: `cd frontend && pnpm install && pnpm run tauri:dev`. This runs `scripts/tauri-auto.js` (GPU autodetect). Alternatives are `./clean_run.sh [debug]` or `clean_run_windows.bat`. The Next dev server runs on port 3118.
- Build: `pnpm run tauri:build` or `./clean_build.sh`. Feature-specific variants are `tauri:build:{cpu,metal,coreml,cuda,vulkan,openblas,hipblas}`. Linux uses `./build-gpu.sh`.
- llama-helper: `cd llama-helper && cargo build --release [--features metal|cuda|vulkan]`. It can be overridden with the `MEETILY_LLAMA_HELPER` env var.
- Tests: Rust `cargo test` has about 250 `#[test]`/`#[tokio::test]` across the summary, whisper, templates and analytics modules (none in database or API). Frontend has 10 `bun:test` files in `frontend/tests/` (hooks and lib). **There is no `test` script in package.json and no test step in CI.** `pr-main-check.yml` only validates the version.
- Signing: `TAURI_SIGNING_PRIVATE_KEY` (`frontend/.env.example`), `scripts/sign-windows.ps1`. macOS uses ad-hoc `signingIdentity: "-"`.

### 1.12 Platform support
- macOS: Apple Silicon DMG is the primary target, with min deployment 14.2 and Metal+CoreML.
- Windows: x64 NSIS/MSI with a Vulkan whisper build in CI (README requires AVX2). CUDA needs a source build.
- Linux: deb and AppImage targets exist. README says build from source.
- Mobile: **none.**

---

## 2. Feature inventory

| Feature | Status | Paths | Notes |
|---|---|---|---|
| Meeting auto-detection | **Stub (backend only, unwired)** | `frontend/src-tauri/src/audio/system_detector.rs`, `audio/system_audio_commands.rs:39-98` | A macOS detector watches CoreAudio `DEVICE_IS_RUNNING_SOMEWHERE` and lists apps using audio (`list_system_audio_using_apps`), emitting `system-audio-started` and `system-audio-stopped`. **No frontend code calls `start_system_audio_monitoring` or listens to these events.** It is a no-op on Windows and Linux (`system_detector.rs:387-418`). There is no meeting-app classifier and no auto-start or prompt. README lists auto-detect as a PRO feature. |
| Calendar integration | **Missing** | none | Only `NotificationType::MeetingReminder` and `show_meeting_reminder` exist (`notifications/manager.rs:160`, `types.rs:168`), and nothing calls them. |
| Recording pause/resume | **Complete** | `audio/recording_commands.rs:1116-1184`, `components/RecordingControls.tsx`, tray menu `tray.rs:334-372` | Works from both the UI and the tray. |
| Live transcript UI | **Complete** | `app/_components/TranscriptPanel.tsx`, `components/VirtualizedTranscriptView.tsx`, `TranscriptView.tsx`, `contexts/TranscriptContext.tsx`, `hooks/useTranscriptStreaming.ts` | Virtualized list, typewriter reveal, "Listening..." indicator, optional confidence indicator, and reload sync via `get_transcript_history`. |
| Speaker diarization | **Missing** | none | `audio/stt.rs` has a `speaker_embedding` field but the file is not compiled (dead). README says diarization is PRO-only and "planned". |
| Me vs them (mic vs system) | **Missing (schema only)** | migration `20251110000001_add_speaker_field.sql`, `audio/pipeline.rs:851-900` | Audio is mixed **before** VAD and STT. The `speaker` column is never populated. Implementing this requires running VAD and STT per channel (or tracking a per-window energy ratio) and passing `source` through `worker.rs` into `TranscriptUpdate` and `api_save_transcript`. |
| User note-taking during meeting | **Missing (schema only)** | migration `20251223000000_add_meeting_notes.sql` | The `meeting_notes` table exists with no commands or UI. The only user input is the post-meeting "Add context for AI summary" textarea (not persisted, `page-content.tsx:60` holds it in local state only). |
| AI merge of user notes with transcript | **Partial (primitive)** | `summary/processor.rs:498-504` | The custom-context textarea is appended as `<user_context>` to the final prompt. This is not Granola-style note enhancement and the context is not saved. |
| Summary templates (built-in) | **Complete** | `frontend/src-tauri/templates/*.json`, `summary/templates/{defaults,loader,types}.rs`, `hooks/meeting-details/useTemplates.ts` | 6 built-ins: **Standard Meeting Notes** (Summary, Key Decisions, Action Items table, Discussion Highlights), **Daily Standup**, **Project Sync / Status Update**, **Retrospective (Agile)**, **Client / Sales Meeting**, **Psychiatric Session Note (SOAP)** (file is misspelled `psychatric_session.json`). Only `daily_standup` and `standard_meeting` are compiled in. The others need the bundled resources. |
| Custom templates | **Partial (file-drop only, no UI)** | `summary/templates/loader.rs:19-30, 95-200` | The loader reads `dirs::data_dir()/Meetily/templates/*.json` (macOS: `~/Library/Application Support/Meetily/templates/`), and these override built-ins with the same id. `api_validate_template` exists. **There is no in-app template editor.** README markets custom templates as PRO. Note the path is `Meetily/`, not the app-identifier data dir. |
| Regenerate summary | **Complete** | `hooks/meeting-details/useSummaryGeneration.ts` (`handleRegenerateSummary`, `handleStopGeneration`), `summary/service.rs`, `summary_processes.result_backup` | Cancel restores the backup. |
| Auto-generate summary after recording | **Complete** | `app/meeting-details/page.tsx:28-119` (`isAutoSummary` in localStorage), `page-content.tsx:160-185` | |
| Summary language / translation | **Complete** | `summary/language_detection.rs`, `summary/metadata.rs`, `components/SummaryLanguageSettings.tsx`, `lib/summary-language*.ts` | Per-meeting override and auto-detect. |
| Edit summary | **Complete** | `components/AISummary/BlockNoteSummaryView.tsx`, `BlockNoteEditor/Editor.tsx`, `api_save_meeting_summary` | Rich editing with dirty tracking and save. |
| AI chat with a meeting / across meetings | **Missing** | none | README: "Chat with Meetings (Coming Soon)" for PRO. |
| Search | **Partial (keyword only)** | `database/repositories/transcript.rs:101` (`WHERE LOWER(t.transcript) LIKE ?`), `api_search_transcripts`, `components/Sidebar/index.tsx:240-318` | Substring match on transcript text plus a client-side title filter. No FTS5, no summary search, no semantic search, no embeddings. |
| Folders / tags / organization | **Missing** | `components/Sidebar/index.tsx` | One hard-coded "Meetings" folder. `meetings.folder_path` is the **audio** folder on disk, not organization. There is no tag or folder schema. |
| People / contacts | **Missing** | none | |
| Action items extraction | **Partial (text only)** | templates (e.g. `standard_meeting.json` Action Items table) | Extracted only as markdown inside the summary. No structured storage, assignee, due date or completion tracking. The legacy `transcripts.action_items` column is unused. About.tsx says "action tracking coming soon". |
| Exports | **Partial (clipboard only)** | `hooks/meeting-details/useCopyOperations.ts` | Copy transcript (plain text with timestamps) and copy summary (markdown from BlockNote) to the clipboard. Opening the meeting folder shows `audio.mp4`, `transcripts.json` and `metadata.json`. **No Markdown, PDF or DOCX file export** (README lists these as PRO). The `save_transcript` command exists in Rust (writes any path) but the frontend never calls it. |
| Sharing | **Missing** | none | |
| Integrations (Slack, Notion, CRM, webhooks) | **Missing** | none | |
| MCP server | **Missing** | none | |
| External API | **Missing** | none (legacy FastAPI only in `backend/`) | |
| Audio import | **Complete (beta flag, default ON)** | `audio/import.rs` (1323 lines), `components/ImportAudio/*`, `hooks/useImportAudio.ts`, `layout.tsx` drag-drop | Formats: mp4, m4a, wav, mp3, flac, ogg, aac, mkv, webm, wma. Gated by `betaFeatures.importAndRetranscribe` (`types/betaFeatures.ts`). |
| Re-transcription | **Complete (beta)** | `audio/retranscription.rs`, `components/MeetingDetails/RetranscribeDialog.tsx` | Choose model and language, re-run on the saved `audio.mp4`. |
| Audio playback synced with transcript | **Missing in UI (backend data ready)** | `components/AudioPlayer.tsx` (**0 bytes**), `hooks/useAudioPlayer.ts` (unused), `transcripts.audio_start_time` and `audio_end_time` | Timestamps are stored per segment and `read_audio_file` plus the asset protocol exist, but no player is mounted and segments cannot be clicked to seek. |
| Editing transcripts | **Missing** | none | Only the meeting title is editable (`api_save_meeting_title`). There is no command to update transcript rows. |
| Crash recovery | **Complete** | `hooks/useTranscriptRecovery.ts`, `components/TranscriptRecovery/`, `services/indexedDBService.ts`, `audio/incremental_saver.rs` | Recovers transcripts from IndexedDB and audio from checkpoints. |
| Onboarding | **Complete** | `components/onboarding/*`, `contexts/OnboardingContext.tsx`, `onboarding.rs` | 4 steps: Welcome, Setup Overview (DB init), Download (Parakeet and a recommended built-in LLM), Permissions (macOS only). Also detects a legacy or Homebrew DB for import. |
| Settings | **Complete** | `app/settings/page.tsx`, `components/{PreferenceSettings,RecordingSettings,TranscriptSettings,SummaryModelSettings,ModelSettingsModal,BetaSettings,DeviceSelection,AudioBackendSelector}.tsx` | |
| Notifications | **Complete** | `notifications/*` | Recording started and stopped notifications with a DND check. Consent is auto-set to true on first launch (`lib.rs:522`). |
| System tray | **Complete** | `tray.rs` | Start, pause, resume and stop, open, settings, check updates. |
| Auto-updates | **Complete (points at Zackriya)** | `tauri.conf.json` `plugins.updater`, `services/updateService.ts`, `UpdateCheckProvider.tsx`, `UpdateDialog.tsx` | Endpoint `https://github.com/Zackriya-Solutions/meeting-minutes/releases/latest/download/latest.json`, minisign pubkey belongs to Zackriya. **Must be replaced (new endpoint plus your own minisign keypair) or disabled in the fork. As shipped, a fork build would offer to install upstream Meetily releases over itself.** |
| Telemetry / analytics | **Complete, opt-in (default OFF)** | `analytics/commands.rs:12-13`, `analytics/analytics.rs`, `lib/analytics.ts`, `components/AnalyticsProvider.tsx`, `AnalyticsConsentSwitch.tsx`, `AnalyticsDataModal.tsx` | PostHog key `phc_ohznXPkRSJYWmrfez9mYxtXv5U5Nekq3iiUts87dJfcr`, host `https://us.i.posthog.com`. A migration forces opt-out (`analyticsDefaultOffMigrationV1`). It tracks many events (meeting, summary, model change, custom prompt length, DAU). |
| Multi-language transcription | **Complete** | `constants/languages.ts`, `LanguagePickerPopover.tsx`, `set_language_preference` | |
| Device hot-swap / BT warning | **Complete** | `audio/device_monitor.rs`, `audio/playback_monitor.rs`, `components/BluetoothPlaybackWarning.tsx` | BT notice link points to the placeholder `github.com/your-org/meetily/...`. |
| Recording-consent notice | **Present** | `components/ComplianceNotification.tsx` (used in Sidebar) | |
| Cloud STT providers | **Stub** | `components/TranscriptSettings.tsx:13,56-58,126-128` | Options are commented out and there is no backend. |
| Gemini LLM | **Stub** | migration `20251229000000_add_gemini_api_key.sql` | Column only. |
| PRO licensing | **Stub / remnant** | `licensing` table, CI secret `MEETILY_RSA_PUBLIC_KEY` in workflows | No code reads it. **No feature gating in the app code.** |
| Mobile | **Missing** | none | |

---

## 3. Code health notes (for extending)

### Where new features plug in
- **New Tauri command:** add a module under `frontend/src-tauri/src/<feature>/` with `commands.rs`, declare `pub mod` in `lib.rs:38-56`, and register it in `generate_handler!` (`lib.rs:612`). Commands take `AppHandle<R>` and `tauri::State<'_, AppState>` for the DB (`state.rs`, `database/manager.rs`). Return `Result<T, String>`.
- **DB changes:** add a timestamped `.sql` file in `frontend/src-tauri/migrations/` (sqlx migrator), a repository in `database/repositories/`, and a model in `database/models.rs`. Ready-made but unused tables: `meeting_notes` (notes), `transcripts.speaker` (me vs them).
- **Speaker / me-vs-them:** modify `audio/pipeline.rs` (`AudioPipeline::run`, around lines 820-920) to run VAD per device or tag windows by dominant-channel RMS, then carry the source through `AudioChunk` into `transcription/worker.rs` (the `TranscriptUpdate.source` field already exists), `TranscriptContext.tsx`, `api_save_transcript` and `transcript.rs`.
- **Transcription engines:** implement `audio/transcription/provider.rs::TranscriptionProvider` and add a branch in `engine.rs`.
- **LLM providers:** add an `LLMProvider` variant and endpoint in `summary/llm_client.rs:215-345`, a model-list command (pattern: `openai/openai.rs`), and a UI option in `ModelSettingsModal.tsx:877`.
- **Chat / RAG:** reuse `summary/llm_client.rs::generate_summary`-style calls. There is no embedding infra yet. The built-in llama-helper protocol only supports `Generate`, `Ping` and `Shutdown`, with no streaming.
- **Meeting detection:** wire `start_system_audio_monitoring` from the frontend (e.g. in `layout.tsx`) and listen for `system-audio-started` (the payload is the list of app names), or extend `system_detector.rs`.
- **Notes UI:** the meeting-details split view (`components/MeetingDetails/MeetingDetailsSplitView.tsx`) and the home recording screen (`app/page.tsx`, `app/_components/TranscriptPanel.tsx`). BlockNote is already a dependency.
- **Exports:** `hooks/meeting-details/useCopyOperations.ts` already builds markdown. Add `tauri-plugin-dialog` save (the plugin is already initialized in `lib.rs:468`) plus the existing `save_transcript` command.
- **Beta flags:** `frontend/src/types/betaFeatures.ts` has a documented pattern.

### Conventions
- Rust uses `anyhow` internally and `Result<_, String>` at the command boundary. Hot paths use the `perf_debug!` and `perf_trace!` macros. Logging is emoji-heavy.
- Frontend state: Context providers plus hooks, `invoke()` direct or the `invokeTauri` wrapper, `listen()` for events, and sonner toasts.
- Naming: "microphone" and "system" for audio devices. Provider ids are strings (`"localWhisper"`, `"parakeet"`, `"builtin-ai"`, `"custom-openai"`, `"claude"`, etc.).

### Testing
- Rust has about 250 unit tests, concentrated in `summary/` (processor 30, service 24) and `whisper_engine`. There are **no tests for the database, API, recording pipeline or import**.
- Frontend has 10 `bun:test` files and no `test` script.
- CI does not run tests (`.github/workflows/pr-main-check.yml` only checks the version).

### Fragile spots
- **Meeting persisted only at stop by the frontend.** If the webview dies mid-meeting, data survives only through IndexedDB plus audio checkpoints. Moving persistence into Rust would be more robust.
- `api_get_meetings` returns all meetings without pagination. Search uses a full-table `LIKE`.
- API keys are stored in plaintext SQLite (`settings` table). There is no OS keychain.
- The `save_transcript` and `read_audio_file` commands accept arbitrary paths. The `fs:read-all` and `fs:write-all` capabilities are broad (`tauri.conf.json`).
- The CSP `connect-src` still lists the legacy ports 5167 and 8178.
- The frontend invokes two unregistered commands (`api_get_auto_generate_setting`, `builtin_ai_get_models_directory`).
- `get_transcription_status` is a stub that returns zeros.
- There are two data-dir conventions. Tauri `app_data_dir` (from the identifier) holds the DB and models. `dirs::data_dir()/Meetily` holds custom templates and some model fallbacks (`whisper_engine.rs:154`, `parakeet_engine.rs:294`, `model_manager.rs:148`). Changing the bundle identifier moves the DB and models but not templates, which effectively orphans existing users' data. That is fine for a fresh fork but needs care.
- Hardcoded app version `'0.4.1'` in `AnalyticsProvider.tsx` and `About.tsx` defaults.
- `reactStrictMode: false` (BlockNote). The ProseMirror version pinning via pnpm overrides and webpack aliases is delicate (`next.config.js`).
- Unused deps: `@remirror/*` and `@tiptap/*` (tiptap is only referenced in `next.config.js` resolution), and the `electron/main.js` "main" field in package.json.
- Large files: `audio/recording_commands.rs` (1697 lines), `audio/import.rs` (1323), `summary/service.rs` (1110), `api/api.rs` (1383), `components/Sidebar/index.tsx` (882), `ModelSettingsModal.tsx`.

### TODO/FIXME hot spots
- Mostly in **dead** `src/audio_v2/*` (16 TODOs, "Phase 3/4" stubs).
- Live code has only a few: `components/DeviceSelection.tsx:285` (monitoring), `audio/system_audio_commands.rs:17`, `audio/capture/microphone.rs:2`, `whisper_engine/parallel_processor.rs:363`, `whisper_engine/system_monitor.rs:113`.
- Overall the TODO density is low. The bigger issue is dead files (see 1.3).

### PRO-gated stubs and upsell UI
- **No runtime feature gating in code.**
- Upsell and PRO references:
  - README "Meetily PRO" sections and LAUNCH20 coupon, with links to `https://meetily.ai/pro/`, `https://meetily.ai/enterprise/` and `https://www.zackriya.com/meetily-subscribe/`.
  - `components/About.tsx`: "Chat with the Zackriya team" CTA opens `https://meetily.zackriya.com/#about`, plus "Built by Zackriya Solutions" and a "Coming soon: on-device AI agents" blurb.
  - Code comments referencing "Pro" policy: `audio/pipeline.rs:19`, `components/DeviceSelection.tsx:78`, `pipeline.rs:1110` test name.
  - The `licensing` table migrations and the `MEETILY_RSA_PUBLIC_KEY` CI env.
  - `docs/pv2.0.png` and `pv2.1.png` (PRO screenshots).

---

## 4. Branding inventory

Counts exclude `.git`, `node_modules`, `Cargo.lock` and `pnpm-lock.yaml`. Matches are case-sensitive.

| Term | Occurrences | Files |
|---|---|---|
| `Meetily` | 169 | 60 |
| `meetily` | 120 | 38 |
| `MEETILY` | 18 | 7 |
| `meeting-minutes` | 41 | 12 |
| `meeting_minutes` | 50 | 14 |
| `Zackriya` | 46 | 16 |
| `zackriya` | 7 | 4 |
| `com.meetily.ai` | 1 | 1 |

Top files by combined hits: `README.md` (38), `backend/*` scripts (about 100 combined, legacy and deletable), `scripts/inject_transcript.py` (9), `notifications/types.rs` (8), `audio/recording_preferences.rs` (8), `database/manager.rs` (7), `components/About.tsx` (6), `PRIVACY_POLICY.md` (6), `.github/workflows/WORKFLOWS_OVERVIEW.md` (6), `components/Logo.tsx` (5), `build/ffmpeg.rs` (5), `frontend/{dev,build}-gpu.*` and `build*.bat/ps1` (3-5 each), `tauri.conf.json` (4).

### Must-change identifiers (functional)
| Item | Location | Current value |
|---|---|---|
| Bundle identifier (also determines the app data dir) | `frontend/src-tauri/tauri.conf.json:5` | `com.meetily.ai` |
| Product name and window title | `tauri.conf.json:3,15` | `meetily` |
| Cargo package | `frontend/src-tauri/Cargo.toml:2,6,7` | `name = "meetily"`, authors `Sujith S`, repo Zackriya |
| npm package | `frontend/package.json:2` | `meetily` |
| **Updater endpoint and pubkey** | `tauri.conf.json:113-118` | Zackriya GitHub `latest.json` and Zackriya minisign key |
| **PostHog analytics key and host** | `frontend/src-tauri/src/analytics/commands.rs:12-13` (default host also in `analytics.rs:48`) | `phc_ohznXPkR...`, `us.i.posthog.com` |
| **Parakeet v3 model download host** | `frontend/src-tauri/src/parakeet_engine/parakeet_engine.rs:127` | `https://meetily.towardsgeneralintelligence.com/models/parakeet-tdt-0.6b-v3-onnx` (Zackriya-controlled. Replace with HF `istupakov/parakeet-tdt-0.6b-v3-onnx` or a self-hosted copy. The exact byte sizes are pinned at lines 107-115.) |
| **FFmpeg build-time download** | `frontend/src-tauri/build/ffmpeg.rs:129-144` | `github.com/Zackriya-Solutions/ffmpeg-binaries/releases/...` (5 URLs) |
| ONNX runtime zip name | `frontend/src-tauri/build/onnxruntime.rs:124` | `meetily-onnxruntime-...` |
| Whisper submodule (legacy) | `/.gitmodules` | `github.com/Zackriya-Solutions/whisper.cpp` |
| DB filenames | `database/manager.rs:56-67,127,147`, `database/commands.rs:63-93` | `meeting_minutes.sqlite` / `.db` |
| Recordings folder | `audio/recording_preferences.rs:46-75` | `meetily-recordings` |
| Data dir names | `summary/templates/loader.rs:27`, `whisper_engine.rs:154`, `parakeet_engine.rs:294`, `summary_engine/model_manager.rs:148`, `notifications/settings.rs:118` | `Meetily` / `meetily` |
| CoreAudio aggregate device name | `audio/capture/core_audio.rs:139` | `meetily-audio-tap` |
| HTTP user agent | `whisper_engine.rs:1144,1657` | `Meetily/<ver>` |
| Temp prefix | `audio/decoder.rs:294` | `.meetily_decode_` |
| Env var | `summary/summary_engine/sidecar.rs:110-258` | `MEETILY_LLAMA_HELPER` |
| Console log stream | `console_utils/console_utils.rs:52,90,132` | `--process meetily` |
| IndexedDB / storage keys | `services/indexedDBService.ts:34`, `lib/analytics.ts:188-191`, `MeetingDetailsSplitView.tsx:6` | `MeetilyRecoveryDB`, `meetily_user_id`, `meetily.meetingDetails...` |
| Homebrew legacy paths | `DatabaseImport/HomebrewDatabaseDetector.tsx:15-16`, `contexts/OnboardingContext.tsx:180` | `/opt/homebrew/var/meetily/...` (can delete the legacy import feature entirely) |
| CI artifact names and secrets | `.github/workflows/{build,build-macos,build-windows,build-linux,build-devtest,release,build-test}.yml` | `meetily-*` prefixes, `MEETILY_RSA_PUBLIC_KEY`, `s3://meetily-updates` (release.yml:154) |
| Update scripts | `scripts/generate-update-manifest-github.js:59,120,167,177`, `scripts/test-update-locally.js:25,108` | Zackriya repo URLs |

### User-visible strings
- `tray.rs:26` tooltip
- `notifications/types.rs:122-192` and `notifications/commands.rs:335,385` (notification titles "Meetily")
- `audio/recording_commands.rs:111` and `hooks/useRecordingStart.ts:18` ("Restart Meetily")
- `app/metadata.ts` and `metadata.tsx` (title)
- `components/{Logo,Info,About,PermissionWarning,PreferenceSettings,TranscriptView,VirtualizedTranscriptView}.tsx`
- `Sidebar/index.tsx:693`
- `onboarding/steps/{Welcome,Permissions,SetupOverview,DownloadProgress}Step.tsx`
- `DatabaseImport/*`
- `build.rs:31` (cargo warning)
- Assets: `frontend/public/{logo.png,logo-collapsed.png,icon_*.png}`, `frontend/src-tauri/icons/*`, `frontend/src/app/favicon.ico`, `docs/*.png`/`gif`

### External links to remove or replace
- `components/About.tsx:26` → `https://meetily.zackriya.com/#about` (Zackriya sales CTA)
- `components/AnalyticsConsentSwitch.tsx:150` → Zackriya `PRIVACY_POLICY.md`
- `components/onboarding/steps/SetupOverviewStep.tsx:101` → Zackriya GitHub repo
- `components/BluetoothPlaybackWarning.tsx:84` → placeholder `github.com/your-org/meetily/...` (broken link)
- `README.md` → meetily.ai, meetily.ai/pro, meetily.ai/enterprise, zackriya.com subscribe, Discord, Reddit and LinkedIn links, LAUNCH20 coupon, trendshift badge
- `PRIVACY_POLICY.md`, `CONTRIBUTING.md`, `LICENSE` (copyright line), `THIRD_PARTY_NOTICES.md`, `docs/BUILDING.md`, `docs/building_in_linux.md`, `docs/GPU_ACCELERATION.md`, `CLAUDE.md`, `frontend/README.md`, `frontend/API.md`

### Telemetry, update and network endpoints summary
1. PostHog `https://us.i.posthog.com` with the Zackriya project key (opt-in, default off).
2. Updater: Zackriya GitHub releases `latest.json` and the Zackriya minisign pubkey.
3. Model downloads:
   - Parakeet v3 from the Zackriya-hosted `meetily.towardsgeneralintelligence.com`.
   - Parakeet v2, Whisper and built-in LLMs from HuggingFace (neutral).
4. Build-time: FFmpeg from Zackriya GitHub. ONNX Runtime from Microsoft GitHub (neutral).
5. CSP allows `https://api.ollama.ai` plus localhost 11434, 5167 and 8178.
