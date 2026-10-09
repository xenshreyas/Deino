# Anarlog (ex-Hyprnote) audio stack: what to port into Deino

Repo: `~/Documents/GitHub/deino-references/anarlog`. All paths below are relative to it.
- The live engine is `crates/listener-core` (ractor actors). Batch transcription and diarization are in `crates/listener2-core`.

## 0. Licensing
- **MIT everywhere except `enterprise/**`**, which is commercial and must **not** be copied.
- Check these before porting:
  - `crates/tcc/swift-lib` is adapted from AudioCap and uses the private TCC API.
  - The DTLN-aec ONNX models (upstream is MIT).
  - The Silero v6.2 model (MIT).
  - The pyannote segmentation model (MIT).
  - `crates/embedding/src/onnx/embedding.onnx` is likely WeSpeaker, which is probably CC-BY-4.0 and needs attribution.
  - `crates/mp3` links **LAME, which is LGPL. Avoid it**: use AAC or Opus or FLAC instead.
  - The test WAVs have unknown provenance.

## 1. System audio: Core Audio process tap via cidre 0.15
- **Tap and aggregate device:** `crates/audio-actual/src/speaker/macos.rs:52-187`.
  - `TapDesc::with_mono_global_tap_excluding_processes(&[])`, then a private aggregate device with `tap_auto_start=false`, name `anarlog-audio-tap` and a UUID uid.
  - An `extern "C"` IO proc gets a `Box<Ctx>` as client data. Cleanup is RAII through the guards.
  - **Deino should exclude its own PID**; Anarlog excludes nothing.
  - There is no stale-device cleanup; `is_private=true` makes the device process-local.
  - The tap device is filtered out of device lists by name (`crates/audio-device/src/macos.rs:119,155-181`, `audio-actual/src/mic.rs:15-25`).
- **Format:** the tap ASBD is converted (F32/F64/I32/I16) into a preallocated scratch buffer of 8192 samples (`speaker/macos.rs:96-186`, `rt_ring.rs:4`).
- **Rate changes:** `tap.asbd()` is polled every 128 empty polls (`speaker/macos.rs:258-277`), then the rubato async resampler is rebuilt (`crates/resampler/src/async_source/dynamic_new.rs:171-279`).
- **Device-change monitor:** `crates/device-monitor/src/macos.rs:14-16, 382-592`, `lib.rs:86-160`, `debounce.rs`.
  - Listens to the default input, default output and the device list.
  - Events go through a bounded `sync_channel` with `try_send` and are debounced per kind (1000 ms).
  - It rolls back failed registrations and uses a `POISONED` flag that leaks the context rather than risk a use-after-free.
- **Restart policy:**
  - `DeviceChangeWatcher` (`crates/listener-core/src/actors/source/mod.rs:96-220`) stops the source on a device change.
  - The supervisor restarts it without using up the restart budget (`session/supervisor.rs:218-232`, `children.rs:24-47`, backoff 1/2/4 s).
  - The generic budget is 3 restarts per 15 s (`crates/supervisor/src/restart.rs`).
- **Headphone routing:** a 2 s poll; it only reacts after two consecutive matching polls (`source/mod.rs:193-220`).
- **Keep-alive:** a silence stream keeps the output awake, filtered out by the app's own PID (`source/mod.rs:250`, `audio-device/src/macos.rs:127-143`).
- **Sleep:** `crates/detect/src/sleep.rs:40-92` (NSWorkspace notifications). Anarlog stops the session on sleep.
- **Bluetooth/HFP:** `crates/audio-device/src/lib.rs:74-319`, `source/stream.rs:92-123`.
  - A Bluetooth **default** input is swapped for a wired mic.
  - A Bluetooth mic the user picked explicitly is set as the system default, to force the HFP handoff, and restored on drop.
  - A `BLUETOOTH_DEFAULT_OWNERS` counter suppresses the device-change restarts this causes.
- **Permissions:**
  - Probe by opening the tap (`plugins/permissions/src/ext.rs:398-419, 552-558`). This is safer than the private TCC preflight.
  - Mic permission via `AVCaptureDevice` (`:381-386, 524-550`).
  - `Info.plist` has both `NSMicrophoneUsageDescription` and `NSAudioCaptureUsageDescription`.

## 2. Microphone: cpal 0.17
`crates/audio-actual/src/mic.rs`
- Device ranking that rejects the tap, monitor and null devices (`:27-78`).
- The stream is built and played on a **dedicated thread** with a 5 s init timeout (`:586-610`) and dropped on the same thread (`:575-639`).
- The error callback sets an `alive` flag (`:492-496`).
- Multichannel input is downmixed to mono in the callback (`rt_ring.rs:12-99`).

## 3. Separate channels, alignment, real-time safety
- **Separate channels:** `CaptureFrame { raw_mic, raw_speaker, aec_mic }` (`crates/audio/src/lib.rs:39-56`). Live STT gets dual-channel audio (`ListenerAudio::Dual`, `pipeline.rs:435-464`).
- **Chunking:** both streams go to 16 kHz in 120 ms chunks (1920 samples) (`audio-utils/src/lib.rs:589-595`).
- **Joiner:** `capture/joiner.rs:5-58` pairs chunks by arrival. `MAX_LAG` is 4 chunks before silence is filled in; each queue is capped at 30.
- **AEC alignment:** a GCC-PHAT lag estimator (`crates/audio-sync/src/estimator.rs`, `probe.rs:83-89` state machine) drives `AecReferenceAligner` delay lines of up to 600 ms (`capture/stream.rs:212-407`).
- **No host timestamps** are used. Deino should add Mach host timestamps (see the Cap notes).
- **Ring buffers:** `ringbuf 0.4` HeapRb SPSC with 65,536 samples.
  - The callback converts into scratch, calls `push_slice`, counts drops in an atomic and wakes the consumer only when a flag is set (`speaker/macos.rs:209-247`, `mic.rs:475-491`).
  - Reader: `async_ring.rs:22-150` (AtomicWaker with a recheck, drop logs rate-limited).
- **Backpressure:** bounded channels with `try_reserve` (`pipeline.rs:23-36, 273-428`).
  - Listener: retry for 1 s, buffer 150 chunks with 5 s of replay.
  - Recorder: retry for 2 s, then stop.

## 4. Echo cancellation
- **DTLN-aec:** two-stage ONNX, block 512 and shift 128 at 16 kHz (`crates/aec/src/onnx/{model,mod}.rs`).
  - **Keep chunk sizes a multiple of 128**; any remainder is silently zeroed (`:266`).
  - It allocates on every call (`:257-269`). Deino must fix that and run AEC off the async workers.
- **Linear residual post-filter:** `capture/stream.rs:548-661`; constants at `:26-35` (correlation 0.12, max gain 1.25, smoothing 0.12, double-talk ratio 0.08).
- **Headphones:** if every running output is headphones, AEC is turned off and the mic is marked isolated (`source/stream.rs:50-90`, `audio-device/src/{lib.rs:38-72, macos.rs:95-109, 271-298}`).
- **Don't copy:** the mic is VAD-masked before recording (`pipeline.rs:147-148`), so quiet speech is lost.

## 5. Resampling and VAD
- **Resampler:** rubato 1.0 `Async::new_poly` (cubic) in `RubatoChunkResampler`, with preallocated queues (`resampler/src/async_source/{dynamic_new,driver}.rs`).
- **Live VAD:** earshot at 16 kHz, plus a 6-frame hangover and an RMS floor of 0.0005 (`crates/vad/src/earshot.rs`, `vad-masking/src/streaming.rs`).
- **Batch VAD:** Silero v6.2, 512-sample chunks with 64 samples of context (`crates/vad/src/silero_onnx/v6.rs`).
- **Batch chunker:** `crates/audio-chunking` uses thresholds 0.5/0.35, 600 ms redemption and 600 ms pre-pad, with chunks of 3 s minimum, 20 s target, 25 s maximum (`vad/session.rs:20-36, 273-292`).
- **Health:** a mic dropout monitor (≥15% exact zeros over 5 s, `pipeline.rs:583-644`).

## 6. Diarization and voiceprints
- **Pipeline:** `crates/pyannote-local`, a CPU port of pyannote 3.1.
  - Segmentation uses 10 s windows with 3 local speakers.
  - Embeddings are 256-dim (`crates/embedding`).
  - Clustering is centroid-linkage agglomerative with threshold 0.7045 (`clustering.rs:241-275`), overridden by speaker-count bounds.
  - **DER metrics and an eval binary over RTTM** (`metrics.rs`, `src/bin/diarization_eval.rs`). Port these as the CAP-03 benchmark harness.
- **Voiceprints:** `crates/voiceprint`.
  - Clean single-speaker spans are chosen from the transcript words.
  - Matching is unique assignment with `MIN_UNIQUE_SCORE=0.62` and `MIN_UNIQUE_MARGIN=0.08`, plus elimination (`matching.rs`).
  - Known voiceprints merge fragmented clusters (`pyannote-local/src/pipeline.rs:340-380`).
- **Channel planning:** `crates/listener2-core/src/batch/diarize.rs:27-134`.
  - Ch0 (mic) is "You" and is not diarized. Ch1 (system) is diarized.
  - **In-person rule:** if 2 or more participants were expected and ch1 had 3 s or less of speech, the mic channel is diarized instead.

## 7. Meeting detection
- **Mic in use:** a `DEVICE_IS_RUNNING_SOMEWHERE` listener on the default input, re-armed when the default changes, with a 500 ms edge debounce and a 1 s app poll (`crates/detect/src/mic/macos/*`).
- **Which app:** per-process `ca::System::processes()` with `is_running_input()`, resolved to the **outermost `.app` bundle**, so browser and Electron helpers map to the parent app. FaceTime call daemons are relabelled (`crates/detect/src/list/macos.rs:84-290`).
- **Policy:**
  - A denylist by category (own app, dictation apps, IDEs, screen recorders, AI assistants) plus user include and ignore lists (`plugins/detect/src/policy.rs`).
  - An app must hold the mic for **15 s** before detection fires, then gets a **10-minute cooldown** per app (`mic_usage_tracker.rs`, `handler.rs:58-90`).
- **Browser meetings via the AX tree:** a URL → platform map for Meet, Teams, Zoom, Webex, Slack and Discord, plus control matching (`crates/detect/src/meeting_ax*`). Also a Zoom mute watcher (`crates/detect/src/zoom.rs`).
- **Auto-stop:** after `micStopped`, confirm for 5 s. Calendar end + 10-minute grace, and an 8-minute network hold (`apps/desktop/src/stt/auto-stop.ts:17-23`, `detect-events.ts:437-476`).
- There is no CMIO camera detection; take that from Muesli.

## 8. Crash-safe recording
`crates/listener-core/src/actors/recorder/chunks.rs`
- **Threads:** a persistence thread owns every file (`:43-67, 582-644`).
- **Chunk publishing:** 60 s chunks are written as `.part`, then `sync_all`, then renamed (`:746-760`). The full-session archive is fsynced before any chunk is published (`:491-520`). Each new chunk re-encodes a 2 s overlap at its head (`:701-744`).
- **Limits:** an in-memory cap of 64 MB, a 256 MB disk reserve and a 256 MB recovery budget, checked every 5 s. A storage health callback reports Delayed, Resumed, DiskLow and DiskOk (`:17-24, 69-74, 789-826`).
- **Recovery:** on startup, orphan `.part` files are decoded, measured and renamed (`:955-1003`, `root.rs:165-185`).
- **Writer limit:** at most 4 concurrent writers (`recorder/mod.rs:48-50`).
- **Deino:** use the same pattern with AAC/Opus fragments (not LAME), with 5 s or shorter flushes so at most ~5 s is lost (FND-05).

## 9. Test harness
- **AEC:** a criterion bench (`crates/aec/benches/aec_bench.rs`) and spectral golden snapshots (`crates/aec/data/snapshots`, `crates/audio-snapshot`, regenerated with `UPDATE_SNAPSHOTS=1`).
- **Sync:** a GCC-PHAT e2e test with synthetic delays (`crates/audio-sync/tests/e2e.rs`).
- **VAD:** a criterion bench (`crates/vad/benches/vad.rs`).
- **Fake audio sources:** `crates/audio-mock` implements `AudioProvider` from fixture files. Env vars select them: `ANLG_MOCK_MIC_AUDIO`, `ANLG_MOCK_SPK_AUDIO`.
  - The trait boundary is `crates/audio-actual/src/lib.rs:227-281`.
  - Fixtures with separate mic and speaker tracks: `crates/data`.
- **Behaviour tests:**
  - Mic-detect policy tests with paused tokio time: `plugins/detect/tests/mic_detection.rs`.
  - Supervisor reliability tests: `listener-core/.../supervisor/reliability_tests.rs`.
  - Persistence fault injection: `chunks.rs:1065-1102`.
- **Other tools:** a DER eval binary (§6) and an Audacity exporter for listening comparisons.

## Gaps Deino must fix rather than copy
- Exclude Deino's own PID from the tap.
- Add a tap-invalidation listener and wake recovery.
- Track mic sample-rate changes.
- Use Mach host timestamps.
- Do not VAD-mask the recording.
- Make AEC allocation-free and run it off the async workers.
- Avoid LAME.
