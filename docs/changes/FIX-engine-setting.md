# FIX-engine-setting · One source of truth for the transcription engine

## Progress log (PARITY_PLAN §7.1)
Owner QA bug: an older `settings.json` said `"engine": "parakeet"` (Parakeet
TDT, never downloaded) while only Parakeet Unified was installed. Recording
failed with "the Me transcription engine could not start", and Settings →
Transcription showed Parakeet Unified. A second report: `"model":
"parakeet-unified-en-0.6b-Q8_0.gguf"` failed with "unknown model".

- **Settings page reads the settings.** The page now renders
  `features/settings/components/engine-settings.tsx` (`EngineSettings`,
  `EnginePicker`): the select and the model manager show
  `settings.transcription.engine`/`.model` from `useSettings()` and every
  choice is a `usePatchSettings()` merge patch. Removed: the legacy
  `components/TranscriptSettings.tsx` (a `useState` copy of the engine,
  seeded from `ConfigContext`'s hard-coded Parakeet Unified fallback and
  re-synced by an effect; choosing an engine changed only that copy), the
  context's `setTranscriptModelConfig`, and `engineOf` (unknown legacy names
  became Parakeet Unified). `providerOf` stays, one way, for the retranscribe
  dialog. While settings load, the page shows "Loading…", never a default.
- **Model IDs in either form.** `ModelFileId::from_setting` reads a setting
  as the catalog ID or its `.gguf` file name (only `.gguf` is cut: IDs
  contain dots); `SttRuntime::select_model` uses it. The UI's `activeModel`
  normalises the same way (`fileIdOf`). `TranscriptionSettings.model`'s doc
  now says it is an ID.
- **Self-heal at startup.** `deino_core::settings::healed_transcription`
  (pure; port `InstalledModels`) switches to an installed Parakeet Unified
  file when the configured engine's model is not installed (an unresolvable
  ID counts as not installed); TDT present, or nothing installed, leaves it.
  The app implements the port with `stt::InstalledSnapshot` (Unified via
  `ModelService::list`, TDT and Whisper via the legacy engines' discovery)
  and runs it once in `composition::stt::attach_stt`, logging the switch
  (engine and model IDs only).
- **Onboarding.** The AI step's Parakeet Unified download, on success, also
  saves `engine: parakeet_unified` with the downloaded file
  (`useTranscriptionModel`).
- **Typed missing-model error.** `deino_recording::engine::ModelMissing`
  (model name, optional alternative); `EngineError::model_missing` /
  `missing_model()`, and `EngineError`'s message is the `ModelMissing` one:
  "The transcription model (Parakeet TDT v3) isn't downloaded. Download it in
  Settings → Transcription or switch to Parakeet Unified." `SttLanes` maps
  `SttSetupError::is_model_missing()` (Unified) and, for the Advanced
  engines, asks the `InstalledSnapshot` (never the legacy error string).
  `TranscriptionEngine::label()` names the engines.

## Follow-ups (PARITY_PLAN §7.2)
- Owner QA: launch with the old `settings.json` (copy, not the real one) and
  check the log line "switching to Parakeet Unified", the page and a test
  recording.
- FND-11/legacy: the Parakeet TDT and Whisper managers are still the
  inherited components (`components/ParakeetModelManager.tsx`,
  `WhisperModelManager.tsx`); their `autoSave` path is no longer used by the
  page and can go when they are ported.
- Choosing an Advanced engine whose model is missing is allowed (the page
  shows its download); the next launch heals it back to Parakeet Unified if
  that is installed. Revisit if the owner wants the select to refuse it.

## Architecture (ARCHITECTURE.md)
- STT / settings: the engine setting is healed once at startup
  (`healed_transcription` over `stt::InstalledSnapshot`, in
  `composition::stt`). A lane whose model is missing fails with the typed
  `deino_recording::engine::ModelMissing`, whose message the user sees.
- Frontend: Settings → Transcription is `features/settings/components/engine-settings.tsx`,
  over `useSettings()` only; `components/TranscriptSettings.tsx` is gone.
