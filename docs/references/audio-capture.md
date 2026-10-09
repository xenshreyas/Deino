# macOS audio capture, real-time safety, crash-safe recording, meeting detection, auto-stop and recording health: reference study for Deino

Scope: PARITY_PLAN §3A, §3C and cards FND-05, CAP-02, CAP-06, CAP-07, CAP-09, CAP-10.
Baseline: `references/audio.md` (the Deino audit). That audit's findings are assumed here and not repeated, except where a recommendation fixes one of them.
All reference paths are relative to `~/Documents/GitHub/deino-references/`. All Deino paths are relative to `frontend/src-tauri/src/`.

## 0. Licences and how each repo can be used

| Repo | Licence (checked) | What Deino may do |
|---|---|---|
| `anarlog` | MIT for everything outside `enterprise/` (`LICENSING.md`, `LICENSE`, "Copyright (c) 2023-present Fastrepl, Inc."). Every file cited below is outside `enterprise/`. | Copy or adapt with the MIT notice kept in `THIRD_PARTY_NOTICES.md`. In practice most of it has to be split anyway to meet the 300-line and 40-line limits. |
| `muesli` | MIT ("Copyright (c) 2026 Pranav Hari"). The only Apache-2.0 part is the vendored `Sources/MuesliCore/Qwen3ASR/` (NOTICE), which is not relevant here. | Swift, so the logic has to be ported to Rust. Tables and thresholds can be reused with attribution. |
| `openwhispr` | MIT ("Copyright (c) 2024 OpenWhispr Team"). | JS and Swift, so port to Rust with attribution. |
| `Handy` | MIT ("Copyright (c) 2025 CJ Pais"). | The Rust real-time callback pattern can be copied with the notice. |
| `Cap` | AGPLv3, except `cap-camera*` and `scap-*`, which are MIT (`LICENSE` lines 1-7, `licenses/LICENSE-MIT`). `crates/recording`, `crates/timestamp` and `crates/audio` are AGPL. | **Reference only, by instruction.** Extract principles and never copy text. The MIT carve-out of `scap-*` is noted for information only. |
| `zed` | `crates/audio` is `GPL-3.0-or-later` (`crates/audio/Cargo.toml`). | Reference only. Nothing to copy. |

---

## 1. Process tap setup and teardown, aggregate naming, stale cleanup, permissions

### Best found
- **anarlog `crates/audio-actual/src/speaker/macos.rs:52-141`** is the closest Rust and cidre analogue to Deino.
  - It builds the tap with `TapDesc::with_mono_global_tap_excluding_processes` (`:54`).
  - It builds a private, tap-only aggregate (`:64-79`) with `is_private=true` and `tap_auto_start=false`, a fixed name (`TAP_DEVICE_NAME = "anarlog-audio-tap"`, `src/lib.rs:18`) and a fresh UUID UID.
  - Teardown is RAII. `SpeakerStream` owns `StartedDevice<AggregateDevice>` and `TapGuard` (`:24-33`). cidre destroys both on drop: `TapGuard::drop` calls `AudioHardwareDestroyProcessTap`, and `AggregateDevice::drop` calls `AudioHardwareDestroyAggregateDevice` (cidre checkout `a9587fa`, `core_audio/hardware_tapping.rs:27-31`, `core_audio/hardware.rs:1684-1693`).
  - The mic enumerator hides the tap device by name (`src/mic.rs:15-25, 37-50`), so it can never be picked as a microphone.
- **muesli `native/MuesliNative/Sources/MuesliNativeApp/CoreAudioSystemRecorder.swift`** has the most field-hardened tap lifecycle.
  - It **excludes its own process** from the global tap, resolving its own object through `kAudioHardwarePropertyTranslatePIDToProcessObject` (`:291-303`, `:700-714`). Deino passes an empty exclude list today (`capture/core_audio.rs:91`), so Deino's own playback and notification sounds leak into the "Them" track.
  - It uses a **stereo** global tap, `isPrivate=true` and `muteBehavior=.unmuted` (`:360-370`).
  - Its aggregate dictionary (`:372-385`) sets `kAudioSubTapDriftCompensationKey: true` and `TapAutoStart: true`.
  - **Stable aggregate UID with one deterministic fallback** (`:312-347`). The reasoning is in the comment: private aggregates are *invisible to enumeration*, so a sweep cannot find a crashed session's device, and a fresh UUID per attempt leaks HAL settings entries. Muesli therefore uses `com.muesli.system-audio-tap`, then `...-fallback`, and logs the failure loudly.
  - `cleanupStaleDevices()` (`:772-811`) destroys any device named "Muesli System Audio". This only catches *public* leftovers, which is consistent with the comment above.
  - Permission handling:
    - `checkSystemAudioPermission()` creates and destroys a test tap (`:682-698`).
    - `requestSystemAudioAccess()` starts a real tap recording to trigger the prompt and polls for 12 s. The doc comment says the prompt appears "the first time you start recording from an aggregate device that contains a tap", and only if `NSAudioCaptureUsageDescription` is present (`:733-762`).
    - The deep link to the "Screen & System Audio Recording" pane is `x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture` (`:764-768`).
- **openwhispr `resources/macos-audio-tap.swift:50-56, 114-164`** is a minimal counter-example. It uses a UUID per run in the UID (`com.openwhispr.audio-tap.<uuid>`) and then waits until `kAudioDevicePropertyDeviceIsAlive` reports the aggregate alive before starting (`waitForAggregateDeviceReady`, `:156-164`). The readiness wait is worth adopting.
- **TCC preflight:** anarlog `crates/tcc/swift-lib/src/lib.swift:1-84` (credited to insidegui/AudioCap) `dlopen`s the private `TCC.framework` and calls `TCCAccessPreflight("kTCCServiceAudioCapture")`. It returns 0, 1 or 2 for granted, denied or never asked (`crates/tcc/src/lib.rs:12-19`). Its permission plugin requests access by briefly probing the speaker while a silence stream plays (`plugins/permissions/src/ext.rs:552-558`). It opens settings through a list of fallback URLs (`ext.rs:259-279`).
- `Info.plist`: anarlog `apps/desktop/src-tauri/Info.plist:5-8` has both `NSMicrophoneUsageDescription` and `NSAudioCaptureUsageDescription`. Deino's `Info.plist` already has both.

### Adopt for Deino
- **Tap:** stereo global tap, **excluding Deino's own process object**, `private=true`, `mute_behavior=unmuted`. Downmix to mono on the worker thread, not in the callback; it is cheap either way, but the callback stays minimal.
- **Aggregate:** private and tap-only (no sub-device list, as Deino already does), with `tap_auto_start=true` and drift compensation on.
  - **Name:** `Deino Audio Tap`.
  - **UID:** stable `app.deino.audio-tap`, with a single fallback `app.deino.audio-tap.fallback` (muesli's reasoning).
  - **Readiness:** after creating the aggregate, wait up to 1 s for `DEVICE_IS_ALIVE` (openwhispr) before `device_start`.
- **Stale cleanup on launch (§3C).** This sweep cannot see private devices. Destroy devices whose *name* starts with `deino-audio-tap`, `Deino Audio Tap` or `meetily-audio-tap` (the legacy name in `capture/core_audio.rs:139`), using `HW_DEVICES` → `kAudioObjectPropertyName`. Document that it only clears public leftovers. A private leftover holding the stable UID is handled by the fallback UID. If both fail, the user sees an error ("restart Core Audio or reboot"), as in muesli `:340`. This meets §3C's intent with what the HAL actually allows.
- **Permissions** go behind a trait, so onboarding and tests can fake them:
  ```rust
  /// Mic, system-audio and camera TCC state as Deino understands it.
  pub trait CapturePermissions: Send + Sync {
      fn microphone(&self) -> PermissionState;          // AVCaptureDevice authorizationStatus
      fn system_audio(&self) -> PermissionState;        // TCC preflight, else Unknown
      fn request_system_audio(&self) -> Result<(), PermissionError>; // start tap ≤12 s, poll
      fn settings_url(&self, kind: PermissionKind) -> &'static [&'static str]; // fallbacks
  }
  pub enum PermissionState { Granted, Denied, NotDetermined, Unknown }
  ```
  - Put the private TCC SPI behind a cargo feature (`private-tcc`, as anarlog does for screen recording). When the feature is off, `system_audio()` returns `Unknown`, and the health monitor turns "callbacks arriving but all zeros for 10 s from the start" into a `SystemAudioPermissionSuspected` event.
  - Deep links, in order:
    1. `...com.apple.settings.PrivacySecurity.extension?Privacy_ScreenCapture`
    2. `...com.apple.preference.security?Privacy_ScreenCapture`
    3. the bare pane
- **Teardown order:** stop the IOProc, then destroy the IOProc ID, then the aggregate, then the tap. With cidre this is drop order, so declare `_device` before `_tap` in the owning struct, as anarlog does (`macos.rs:24-33`). Keep the `Box<Ctx>` alive until after the device stops.

### Copy vs reimplement
- **anarlog `speaker/macos.rs`:** MIT. Copying the dictionary construction and the IOProc skeleton is fine, with the notice. Change the name, UID, stereo setting and self-exclusion.
- **muesli UID strategy, readiness wait and request flow:** reimplement in Rust from the description.
- **anarlog TCC Swift shim:** MIT, but it uses private SPI. Reimplement as about 30 lines of `dlopen`/`dlsym` in Rust (`libc`) behind the feature flag, so no Swift package is needed.

---

## 2. Real-time-safe callbacks, worker threads, backpressure

### Best found
- **Handy `src-tauri/src/audio_toolkit/audio/recorder.rs` is the gold standard for the mic.**
  - `build_stream` (`:426-481`):
    - Creates an `rtrb::RingBuffer` sized `rate × 2 s` (`AUDIO_RING_SECONDS`, `:33`).
    - **Pre-faults the ring pages** by writing and reading the full capacity once before starting (`:441-454`).
    - The cpal error callback **only sets an `AtomicBool`** (`:470-475`), because "Error callbacks may share the platform audio thread".
  - `write_input_to_ring` (`:483-547`) is documented as "allocation-free, wait-free, and free of locks, logging, clocks, and system calls".
    - It uses `producer.write_chunk_uninit(n).fill_from_iter(...)` with the downmix or channel selection inline.
    - It counts drops with `fetch_add(Relaxed)`.
    - It has a pause handshake made only of atomics.
  - `run_consumer` (`:914-1043`) runs on a plain `std::thread`.
    - It **polls** the ring every 10 ms (`CONSUMER_POLL_INTERVAL`, `:34`) via `cmd_rx.recv_timeout`, and never wakes anything from the RT thread.
    - It drains in bounded 50 ms slices (`MAX_DRAIN_CHUNK`, `:35`).
    - Logging and recovery happen there.
- **anarlog `crates/audio-actual`** applies the same discipline to the tap:
  - `rt_ring.rs:12-140` has `push_interleaved_downmix_to_mono_ringbuf` and `convert_and_push_to_ringbuf`. They use a preallocated scratch buffer (`DEFAULT_SCRATCH_LEN = 8192`), push into a `ringbuf::HeapProd`, and return `PushStats{pushed, dropped}`.
  - `speaker/macos.rs:96-134, 209-247`: the IOProc reads `AudioBufList` in place, converts any of the f32, f64, i32 or i16 formats, and pushes.
  - **However, it wakes the consumer from the RT thread:** `AtomicWaker::wake()` (`macos.rs:229-231`, `mic.rs:483-486`). That calls into the tokio scheduler, which may lock, so it is not strictly RT-safe.
  - `async_ring.rs:91-111` logs drops from the consumer side, rate-limited to once per second. That is the right place.
- **Backpressure downstream:**
  - anarlog's recorder enqueues with `try_send` into a bounded queue (`WRITE_QUEUE_CAPACITY = 32`, `listener-core/src/actors/recorder/mod.rs:48, 171-195`). The result is `Accepted | Backpressured | Closed`, so the producer decides.
  - The persistence layer then buffers encoded bytes in memory, up to `AUDIO_BUFFER_LIMIT_BYTES = 64 MiB`, while the disk is slow or unavailable (`chunks.rs:17-30, 663-690`). A slow disk delays saving instead of dropping audio.
- **Anti-patterns:**
  - muesli's IOProc allocates `Data` and dispatches async on every buffer (`CoreAudioSystemRecorder.swift:398-424, 444-458`).
  - zed takes a `parking_lot::Mutex` around WebRTC APM in the stream path (`crates/audio/src/audio_pipeline/echo_canceller.rs:10-37`).
  - Deino today does the same (see the audit).

### Adopt for Deino
- One RT contract shared by both sources. It contains only atomics and an SPSC producer, all allocated before `start()`:
  ```rust
  /// Everything an RT callback may touch. Built before start; never resized.
  pub struct RtSink {
      samples: rtrb::Producer<f32>,        // mono, native rate, 2 s capacity, pre-faulted
      stamps: rtrb::Producer<BlockStamp>,  // one per callback, 256 slots
      counters: Arc<RtCounters>,
      scratch: Box<[f32]>,                 // downmix scratch, 8192 frames
  }
  #[derive(Clone, Copy)]
  pub struct BlockStamp { pub host_time: HostTime, pub frames: u32 }
  #[derive(Default)]
  pub struct RtCounters {
      pub callbacks: AtomicU64, pub frames: AtomicU64,
      pub dropped: AtomicU64, pub stream_error: AtomicBool,
  }
  impl RtSink {
      /// RT-safe: no alloc, no lock, no log, no syscall. Returns frames dropped.
      pub fn push_interleaved<S: Copy + ToSample<f32>>(
          &mut self, data: &[S], channels: usize, host_time: HostTime) -> u32;
  }
  ```
- The callback body is a single `sink.push_interleaved(...)` call.
  - The system IOProc uses `input_time.host_time` from the `AudioTimeStamp` argument.
  - The mic callback uses `InputCallbackInfo::timestamp().capture`, converted to host time. On macOS cpal exposes `StreamInstant` host time; Cap's `crates/timestamp/src/macos.rs:56-60` shows that the conversion exists.
  - **Remove from today's IOProc:** `nominal_sample_rate()`, the `Mutex` waker and all logging (audit items).
- **Never wake from RT.** Each track has a `conditioner` thread that polls every 10 ms (Handy's model). At 10 ms granularity the latency cost is negligible for STT and storage.
- **Backpressure policy, in order of priority:**
  1. **RT ring full:** drop and count. Never block. The health monitor reports when more than 0.5% of frames were dropped in a 5 s window.
  2. **Conditioner to writer:** bounded `crossbeam_channel` with about 8 s of blocks. The writer thread itself holds a byte-bounded in-memory queue (64 MiB, as in anarlog) while the disk is slow. Audio is never dropped silently. A full queue raises `HealthEvent::StorageBackpressure`, then `AudioLoss`.
  3. **Conditioner to STT lane:** bounded. When full, **drop the oldest** STT block and count it. The transcript can be redone by CAP-08's post-pass, but audio cannot.
  4. **Conditioner to level meter:** `AtomicU32` holding the RMS as f32 bits. The pill reads it at 30 fps (Handy throttles to about 30 fps; `overlay.rs:730-767`).

### Copy vs reimplement
- **Handy `write_input_to_ring` and the pre-fault block (MIT):** copy and adapt, with the notice.
- **anarlog `rt_ring.rs` format-conversion helpers (MIT):** copy and adapt to `rtrb`.
- **Cap, zed:** nothing taken.

---

## 3. Default-device changes, sleep and wake, sample-rate changes, tap invalidation, Bluetooth HFP

### Best found
- **anarlog `crates/device-monitor/src/macos.rs`** listens to system-object `HW_DEFAULT_INPUT_DEVICE`, `HW_DEFAULT_OUTPUT_DEVICE` and `HW_DEVICES` (`:13-17`).
  - The listener (`:383-433`) only sends an enum on a bounded `sync_channel` and sets atomics. It reads `is_headphone_from_default_output_device()` for output changes.
  - A debounce of `DEFAULT_DEBOUNCE_DELAY = 1000 ms` collapses bursts (`lib.rs:87-89`, `debounce.rs`).
  - If unregistering fails, the callback context is deliberately leaked rather than risk use-after-free. The listener generation becomes "poisoned" (`:19` and `crates/detect/src/mic/macos/mod.rs:24-104`).
- **anarlog restart policy** (`crates/listener-core/src/actors/source/mod.rs:96-222`):
  - A default input or output change restarts the source, **except while a Bluetooth HFP handoff owns the defaults** (`device_switch_restarts_source`, `:184-191`, backed by `audio-device/src/lib.rs:187-189`).
  - `OutputRoutingTracker` (`:193-222`) needs **two consecutive 2 s polls** before a change in the speakers-versus-headphones verdict restarts capture, so a one-off system sound cannot bounce it.
  - Capture keeps a **silent output stream** open while recording (`:250`, `audio-actual/src/lib.rs:43-66`). The comment at `audio-device/src/macos.rs:127-129` calls it "the silence stream that keeps the default output awake".
- **muesli's counterpoint** (`CoreAudioSystemRecorder.swift`, comment near `:880-886`): "The tap is a global process mix — upstream of any output device — so route changes need NO rebuild (proven live: the tap rode an AirPods connect/case cycle untouched)." The default-output listener (`:813-838`) only **timestamps** route churn.
  - Rebuilds happen only on *positive* failure evidence, with bounded backoff `[2 s, 5 s]` (`RebuildRetryPolicy`, `:62-75`).
  - `MeetingAudioRecoveryDeadlines.swift:6-71` does sparse post-event checks at 3, 8, 20, 40 and 60 s instead of idle polling.
  - `AudioRouteController.swift:345-360` treats a burst of Bluetooth route events as one episode.
- **Sleep and wake:**
  - anarlog `crates/detect/src/sleep.rs:1-107` subscribes to `NSWorkspaceWillSleepNotification` and `NSWorkspaceDidWakeNotification` through `objc2-app-kit` blocks on a dedicated thread, and removes the observers on drop.
  - The app stops recording on sleep (`apps/desktop/src/stt/detect-events.ts:466-470`).
  - openwhispr detects sleep **without** notifications too: a tick gap longer than `TICK_GAP_MS = 5 s` resets every silence window (`src/helpers/meetingAutoEndController.js:17-19, 41-59`).
- **Sample-rate changes:**
  - anarlog polls `tap.asbd()` every 128 empty polls (`speaker/macos.rs:258-272`). Its resampler drains and rebuilds when the source rate changes (`crates/resampler/src/async_source/dynamic_new.rs:96-108, 171-182, 216-280`).
  - cidre exposes `TAP_FORMAT` (`hardware_tapping.rs:66`) and `DEVICE_NOMINAL_SAMPLE_RATE` (`hardware_base.rs:682`), so a **listener** is possible instead of polling.
- **Invalidation:** cidre has `DEVICE_IS_ALIVE` (`hardware_base.rs:632`) and `DEVICE_PROCESSOR_OVERLOAD` (`hardware.rs:729`). None of the references listen to `IS_ALIVE` on the aggregate. That gap is worth closing.
- **Bluetooth HFP:**
  - anarlog `audio-device/src/lib.rs:43-73` (`headphone_only_output`) and `:81` (`wired_input_replacing_bluetooth_default`): when the default input is a Bluetooth headset, open a wired mic instead (built-in first, then USB).
  - `listener-core/src/actors/source/stream.rs:80-125` turns AEC off when output is headphone-only and swaps the Bluetooth default mic to wired.
  - The headphone heuristics are in `audio-device/src/macos.rs:95-117, 271-298` (terminal types `HEADPHONES` and `HEADSET_MIC`, Bluetooth transport, name hints).
  - muesli `AudioRouteController.swift:535-548` (`preferredMeetingInputDeviceID`) prefers the built-in mic for meetings, unless the user picked one or it equals the default.

### Adopt for Deino
- **`devices/monitor.rs`** is one Core Audio listener thread. It emits typed events on a bounded channel and nothing else happens in the callbacks:
  ```rust
  pub enum DeviceEvent {
      DefaultInputChanged, DefaultOutputChanged, DeviceListChanged,
      DeviceDied { device: DeviceUid },        // DEVICE_IS_ALIVE → false (mic or aggregate)
      RateChanged { track: Track, hz: SampleRate }, // NOMINAL_SAMPLE_RATE / TAP_FORMAT
      Overload { track: Track },               // DEVICE_PROCESSOR_OVERLOAD
  }
  pub trait DeviceEvents: Send + Sync {
      fn subscribe(&self, tx: crossbeam_channel::Sender<DeviceEvent>) -> Result<Subscription, DeviceError>;
  }
  ```
  Debounce for 1 s on the supervisor side, which is pure and testable with `Clock`, not inside the callback.
- **`devices/power.rs`** observes `NSWorkspace` sleep and wake (port anarlog `sleep.rs`) and emits `PowerEvent::{WillSleep, DidWake}`. The supervisor also runs openwhispr's **tick-gap check** as a fallback (more than 5 s between 1 Hz ticks means the machine was asleep).
- **Restart policy (supervisor). Restart only the affected track, never both:**

  | Event | Mic track | System track |
  |---|---|---|
  | `DefaultInputChanged` (user on "System default") | restart after a 1 s debounce, unless a Bluetooth handoff is in progress | – |
  | `DefaultOutputChanged` | – | **no rebuild** (muesli finding). Record the time as a route episode for health. Re-evaluate the headphone verdict with two consecutive polls (anarlog). |
  | `DeviceDied` | restart on the default device and emit `HealthEvent::DeviceLost` | rebuild the tap with backoff `[2 s, 5 s]`, then watchdog episodes (§7) |
  | `RateChanged` | conditioner rebuilds its resampler (no restart) | same |
  | `WillSleep` | CAP-07 stops the recording (finalize writer) | same |
  | `DidWake` | if still recording (auto-stop off): restart both and let the clock fill the gap with silence | same |
- **Sample-rate changes never reach the writer.** The conditioner reads the new rate from the `RateChanged` event, or detects it from a frame-count versus host-time mismatch over 1 s. It drains the old resampler, builds a new preallocated one, and keeps writing at 48 kHz.
- **Bluetooth HFP (§3C):**
  - On start, and on each `DefaultInputChanged`, classify the route (`devices/route.rs`). HFP is suspected when the input is Bluetooth and its nominal rate is 24 kHz or less (HFP runs at 8 or 16 kHz).
  - If the user is on "System default" and a built-in mic exists, open the built-in mic instead (anarlog and muesli). Otherwise warn once with "Your headset is in call mode; audio quality is reduced".
  - Never change the system default device. anarlog's `BluetoothInputActivation` does (`lib.rs:207+`); Deino should not, because §3C forbids exclusive or intrusive behaviour.

### Copy vs reimplement
- **anarlog device-monitor listener and leak-on-failed-unregister guard (MIT):** adapt and split into files of 300 lines or fewer.
- **anarlog `sleep.rs` (MIT):** copy almost verbatim.
- **anarlog headphone and Bluetooth heuristics (MIT):** copy the classification functions.
- **muesli "no rebuild on route change" rule and backoff schedule:** reimplement.

---

## 4. Mic and system as separate, time-aligned tracks: alignment and drift

### Best found
- **anarlog keeps the tracks separate end to end.**
  - `CaptureFrame { raw_mic, raw_speaker, aec_mic }` (`crates/audio/src/lib.rs`, struct `CaptureFrame`). The stored file is **stereo with mic on L and speaker on R** (`listener-core/src/actors/recorder/disk.rs:331-342`, `chunks.rs:45-66`).
  - Alignment is by **chunk pairing**. `Joiner` (`audio-actual/src/capture/joiner.rs:5-58`) pairs 256-sample chunks, both already resampled to 16 kHz. If one side is more than `MAX_LAG = 4` chunks behind, it emits silence for that side. Queues hold at most 30 chunks.
  - **Lag and drift are measured, not assumed:**
    - `crates/audio-sync/src/probe.rs:52-75` runs a GCC-PHAT lag estimator (`estimator.rs`) with a Searching, Acquiring, Locked, Holdover, Lost state machine (`:83-90`).
    - `drift.rs:1-39` smooths the drift in ppm with an EMA (0.8/0.2).
    - Measurements are used only to align the AEC reference (`capture/stream.rs:212-407`, maximum lag 600 ms). Storage and STT stay raw.
- **Cap is the best principle (reference only):**
  - Every source frame carries a **host timestamp** (`mach_absolute_time` from `AudioTimeStamp` or cpal's `StreamInstant`; `crates/timestamp/src/macos.rs:7-60`).
  - The `MasterClock` maps it onto a single session timeline (`master_clock.rs:17-115`).
  - The per-source remap (`master_clock.rs:260-340`):
    - trusts the source cadence when it stays within `MAX_TS_VAR_NS = 2 s` of wall time;
    - smooths jitter below `TS_SMOOTHING_THRESHOLD_NS = 70 ms`;
    - **hard-resets** after a larger discontinuity;
    - never lets output time go backwards.
  - The mixer **inserts silence for gaps** larger than the buffer timeout (`sources/audio_mixer.rs:566-600`). It scales its timeouts for wired and wireless sources (`:19-30`).
- **muesli** aligns by **sample count per track** (`MeetingChunkTimingTracker.swift:16-58`). For the mixed file it mixes `min(mic, system)` pending samples (`MeetingRecordingWriter.swift:173-190`). That is simple, but it drifts when one side drops frames.

### Adopt for Deino
**Rule: sample index is time.** Each stored track is 48 kHz mono, and sample *n* of `mic.m4a` and of `system.m4a` both mean `t0 + n/48000` of the session. The 50 ms acceptance target in FND-05 then holds by construction, and mixing is a plain sum at finalize.

```rust
/// Maps host-timestamped native-rate blocks onto the session timeline (48 kHz frames).
pub struct TrackClock {
    t0: HostTime,               // session start, shared by both tracks
    rate: SampleRate,           // native rate of the current source
    written: Frames,            // frames already emitted on the timeline (48 kHz)
    paused_total: Nanos,        // accumulated pause time, subtracted from host deltas
    drift: DriftEstimator,      // EMA of (observed - expected), ppm
}
pub enum Placement { Contiguous, Gap(Frames), Overlap(Frames) }
impl TrackClock {
    /// Decides where a block lands; pure, unit-tested with synthetic stamps.
    pub fn place(&mut self, stamp: BlockStamp) -> Placement;
    /// Ratio correction (±200 ppm cap) fed to rubato's `set_resample_ratio_relative`.
    pub fn ratio_correction(&self) -> f64;
}
```
- **Gap** (expected minus observed more than 20 ms; this covers restarts, sleep and ring drops): the conditioner emits that many zero frames first (Cap's silence insertion).
- **Overlap** (more than 20 ms early, which is rare, for example just after a rebuild): drop the overlapping frames.
- **Within ±20 ms:** trust the sample count (Cap's "preserve cadence"). Feed the slow drift into the async resampler as a relative ratio, so it is corrected continuously instead of in 20 ms jumps. The drift is about 50 ppm between the tap clock and a USB mic, which is about 3 ms per minute (anarlog's drift test: `drift.rs:47-62`).
- **Both tracks share `t0`,** taken just before either source starts. The first block of each track is placed relative to `t0`, so a track that starts late begins with silence.
- **The STT lane** gets the same samples at 16 kHz, with a timeline offset in milliseconds. Segments carry `start_ms` and `end_ms` on the shared timeline, so Me and Them merge correctly (CAP-02).
- **Echo and bleed (CAP-02):** anarlog's approach is DTLN-aec ONNX (`crates/aec/src/onnx/model.rs`, 512-sample blocks with a 128-sample shift) plus GCC-PHAT reference alignment. It is too heavy for the CPU budget of 1.6× in CAP-02. Start with the plan's text and energy heuristic. Use anarlog's `SyncProbe` *idea* (cross-correlation of mic against system at lags of 0 to 600 ms) as the "mic energy correlated with system" test, and skip it when the route is headphone-only (anarlog `resolve_capture_settings`, `crates/listener-core/src/actors/source/stream.rs:80-90`).

### Copy vs reimplement
- **Cap's timeline and remap rules:** reimplement from the principle (AGPL, reference only).
- **anarlog `drift.rs` EMA (MIT):** copy. It is 39 lines.
- **anarlog `audio-sync` GCC-PHAT (MIT):** copy into a later CAP-02 bleed module if the correlation test is needed.

---

## 5. Crash-safe recording, encoding and file layout

### Best found
- **anarlog `crates/listener-core/src/actors/recorder/chunks.rs` is the best design in the references, though not the best format for Deino.** It has two writers per session:
  - an **archive** (`audio.mp3`, the whole meeting);
  - **recovery chunks** of 60 s (`CHUNK_SAMPLES = 16 kHz × 60`, `:17`), written as `.part` and renamed to `<capture>-<start>-<end>-<audio_start>.mp3` only **after they are durable** (`:743-761`).

  The format is a streaming LAME MP3 (`crates/mp3/src/encoder.rs`, 128 kbps stereo). MP3 frames are self-synchronising, so a truncated file still decodes.

  A separate **persistence thread owns every file** (`Persister`, `:229-560`):
  - It writes on a 1 s cadence (`WRITE_INTERVAL`, `:22`).
  - It `fsync`s, and publishes a chunk only after the archive covering the same audio is durable (`:489-506`).
  - It retries every 5 s while storage is unhealthy (`:23`).
  - It buffers up to 64 MiB in memory during outages (`:20, :663-690`).
  - It checks disk space every 5 s, with a 256 MiB reserve and budget (`:18-21, 789-826`).
  - It emits `StorageHealth::{Delayed, Resumed, DiskLow, DiskOk}` (`:69-75`).

  Each new chunk replays the last 2 s of history as overlap (`:701-741`). On startup, `recover_partial_chunks` (`:955-1003`) decodes each `.part` to measure its playable tail and renames it. The header of the legacy WAV path is flushed and fsynced before any audio arrives (`disk.rs:99-108`).
- **Cap (reference only)** writes fragmented MP4 (`output_pipeline/macos_fragmented_m4s.rs`) with **2 s segments** (`:185`) and a JSON **manifest** of fragments (`fragmentation/manifest.rs:1-58`). The manifest is written atomically: temp file, `fsync`, rename, `fsync` the directory (`fragmentation/mod.rs:11-29`).
  - `RecoveryManager::find_incomplete` and `is_m4s_complete` (`recovery.rs:285, 849`) recover complete fragments only.
  - `DiskSpaceMonitor` polls every 2 s (`core.rs:532`).
- **muesli** writes a WAV header with `dataSize: 0`, appends PCM, and patches the header at stop (`MeetingRecordingWriter.swift:50-120`, `PCMChunkRecorder.swift:70-99`). It transcodes to M4A (AAC) or keeps WAV at the end (`:23-47, 134-168`), and keeps a `.recovery.json` beside the file (`:7-21`). There is no periodic `fsync`, so after a crash the header is wrong but the PCM can be salvaged. This is weaker than anarlog's design.
- **Deino today** writes 30 s mp4 checkpoints and joins them with ffmpeg concat (audit), so a crash can lose up to 30 s.

### Encoding choice for Deino

| Option | Crash behaviour | Size, 48 kHz mono | WKWebView playback | Decoding for STT re-pass | Verdict |
|---|---|---|---|---|---|
| **AAC-LC in fragmented MP4 (`.m4a`)** | readable up to the last fragment (1–2 s) | about 29 MB per hour at 64 kbps | native | symphonia `isomp4`+`aac` (already in Deino); **verify fMP4 (moof) support with a fixture**, falling back to ffmpeg | **Choose this.** It matches FND-05's `mic.m4a`/`system.m4a` and plays natively. |
| Opus in Ogg | each page is self-delimiting; very robust | about 14 MB per hour at 32 kbps | uncertain in WebKit for Ogg | symphonia has no Opus decoder, so ffmpeg is needed | Good codec, but awkward for playback and decoding |
| FLAC | frame-synchronised; robust | about 100–150 MB per hour | WebKit supports FLAC | symphonia supports it | Lossless, but too big to keep |
| MP3 (anarlog) | robust | about 29 MB per hour at 64 kbps | native | symphonia supports it | Fine, but AAC is the better codec at the same rate and is what FND-05 names |
| WAV/PCM append | header patch needed; robust if the size is derived from length | about 345 MB per hour | native | trivial | Use only for tests and fixtures |

### Adopt for Deino
- **File layout** for each meeting directory, `<app_data>/meetings/<meeting_id>/`:
  ```
  recording.json                 # manifest: atomic write (tmp+fsync+rename+dir fsync), every 5 s and on events
  audio/mic.001.m4a.part         # fragmented AAC, 1 s fragments; renamed to .m4a on clean close
  audio/system.001.m4a.part
  audio/mic.002.m4a.part ...     # new segment only on Resume-after-stop (CAP-07) or encoder restart
  mic.m4a  system.m4a  audio.mp4 # produced by pipeline stage `finalize_audio` (FND-08 step 10)
  ```
  - `recording.json` fields: `version`, `meeting_id`, `t0_host_ns`, `started_at`, `sample_rate=48000`, `tracks[{track, segments[{file, first_frame, frames_written, closed}]}]`, `pauses[{from_frame, to_frame}]`, `status: Recording|Stopping|Closed`, and the last `health` summary.
  - Write the frame counts from what the writer thread has handed to the encoder and the encoder has flushed. Never write them ahead.
- **Mixed `audio.mp4` is derived at finalize** from the two aligned tracks (sum with −6 dB and a soft limiter, or `ffmpeg amix=normalize=0`). There is no third live encoder. Checkpoint recovery "for all three" (FND-05) is satisfied because the mix is a pure function of the two recovered tracks. Note this as a deliberate deviation in the FND-05 ADR.
- **Encoder backend.** Use one trait and pick the backend in the composition root:
  ```rust
  /// Turns 48 kHz mono f32 into a crash-tolerant file. Implementations own no threads.
  pub trait TrackEncoder: Send {
      fn append(&mut self, samples: &[f32]) -> Result<(), EncodeError>;
      /// Push buffered audio to the OS (not necessarily fsync). Called every 1 s.
      fn flush(&mut self) -> Result<(), EncodeError>;
      fn close(self: Box<Self>) -> Result<ClosedSegment, EncodeError>;
  }
  pub trait EncoderFactory: Send + Sync {
      fn open(&self, path: &Path, track: Track) -> Result<Box<dyn TrackEncoder>, EncodeError>;
  }
  ```
  - **Default, `FfmpegFragmentedAac`:** reuse the existing `ffmpeg-sidecar` dependency. It pipes `f32le` into ffmpeg via stdin with `-c:a aac -b:a 64k -f mp4 -movflags +empty_moov+default_base_moof+frag_keyframe -frag_duration 1000000 -flush_packets 1`. Bonus under `kill -9`: the ffmpeg child sees EOF on stdin and finishes its last fragment itself. *Validate this with the FND-05 test; the ffmpeg flag set must be checked with ffprobe on a truncated file.*
  - **Later, `AvAssetWriterAac`:** in-process `AVAssetWriter` (cidre `av`) with `movieFragmentInterval = 1 s`. It removes the sidecar and uses the hardware AAC encoder. Write an ADR when switching.
  - **Tests, `PcmEncoder`:** raw `f32le` append, for unit and contract tests.
- **Durability.** Port anarlog's split: the **writer thread encodes, and a persister handles durability**.
  - Every 1 s: flush the encoders. Every 5 s: `fsync` the `.part` files and rewrite `recording.json` atomically. That bounds `kill -9` loss to about 1–2 s (the fragment) and power-loss loss to about 5 s (FND-05 allows about 5 s).
  - While the disk is unhealthy, keep up to 64 MiB of PCM in memory and retry every 5 s (anarlog's numbers).
  - Disk check every 5 s: `DiskLow` below 2 GB (CAP-10), `DiskExhausted` below 256 MiB, at which point recording stops cleanly with a health event.
- **Recovery at launch (FND-05).** A meeting with `status='recording'` in the DB, or a `recording.json` with `status != Closed`:
  1. For each `.part`, probe the playable duration (ffprobe or symphonia).
  2. Truncate the manifest's frame counts to what is actually playable, and rename `.part` to `.m4a`.
  3. Mark the meeting `processing` and enqueue `finalize_audio`.

  This is anarlog's `recover_partial_chunks` (`chunks.rs:955-1003`) combined with Cap's "complete fragments only".

### Copy vs reimplement
- **anarlog `Persister`, `TargetState` and the `.part`→rename protocol (MIT):** port the design. The code must be split into `writer/persister.rs` (300 lines or fewer) and the tests kept. It uses MP3; swap in `TrackEncoder`.
- **Cap's atomic manifest and complete-fragment rule:** reimplement from the principle (about 25 lines; the pattern is standard).
- **muesli:** nothing beyond the idea of a `.recovery.json` beside the audio.

---

## 6. Meeting detection: process objects, camera, classification, false positives

### Best found
- **Per-process mic use:**
  - **openwhispr `resources/macos-mic-listener.swift` is the best event-driven design:**
    - a listener on `kAudioHardwarePropertyProcessObjectList` plus a listener on **each process object's `kAudioProcessPropertyIsRunningInput`** (`startProcessMonitoring`, `:256-288`);
    - a `reconcile` that diffs the process objects and active PIDs and emits `MIC_START`/`MIC_STOP` (`:290-330`);
    - snapshot code that tolerates the race where a process vanishes between enumeration and the property read (`prepareProcessSnapshot`, `:219-254`);
    - **exclusion of `com.apple.CoreSpeech`**, "since it also processes ordinary playback" (`:244-246`);
    - a fallback to aggregate `DeviceIsRunningSomewhere` mode on older systems (`:388-530`).
  - anarlog polls the same data every 1 s (`crates/detect/src/mic/macos/mod.rs:18`, `app.rs:30-68`) using cidre's `System::processes()` and `is_running_input()` (`list/macos.rs:84-106`).
    - It filters its own app by bundle ID, name and path (`list/mod.rs:24-92`).
    - It **relabels Apple's call daemons** (`avconferenced`, `callservicesd`) as "FaceTime" or "iPhone Call", depending on which app is frontmost or running (`list/macos.rs:11-16, 108-151`).
    - It debounces mic edges by 500 ms on the trailing edge (`mic/macos/state.rs:31, 37-84`).
  - muesli `AudioProcessAttributionCollector.swift:33-113` reads `kAudioProcessPropertyBundleID`, `PID`, `IsRunningInput`/`IsRunningOutput` and `kAudioProcessPropertyDevices` (input scope). `refreshTrackedProcesses` (`:62-81`) re-reads only known objects, so it does not enumerate everything each time.
- **Camera:** muesli `CameraActivityMonitor.swift:74-253` is the only CoreMediaIO implementation.
  - It enumerates `kCMIOHardwarePropertyDevices`, filtered to devices that have input streams (`:226-252`).
  - It puts a listener on `kCMIOHardwarePropertyDevices` (hot-plug) and a per-device listener on `kCMIODevicePropertyDeviceIsRunningSomewhere`, using the wildcard scope and element (`:108-187`).
  - Its state check ORs all devices (`:191-220`).
  - It deliberately avoids `AVCaptureDevice` discovery, because that installs main-queue HAL listeners that stalled on AirPods (`:70-73`).
  - cidre has **no CMIO module** (checked at `a9587fa`).
- **Classification and false-positive avoidance:**
  - muesli `MeetingDetector.swift`:
    - Dedicated apps: `us.zoom.xos`, `us.zoom.ZoomPhone`, `com.apple.FaceTime`, `com.microsoft.teams2`/`teams`, `com.tinyspeck.slackmacgap`, `com.webex.meetingmanager`, `com.cisco.webexmeetingsapp` and `net.whatsapp.WhatsApp` (`:65-75`).
    - "Weak" apps that run in the background, Slack and WhatsApp, need extra evidence (`:80-90`).
    - Browsers only count when they are **frontmost** or have a meeting URL (`:94-100, 340-348`).
    - **Camera alone is not a meeting** (Photo Booth, scanning), so it requires camera plus mic (`:190-201`).
    - The priority order is camera+mic, then calendar+mic, then foreground app, then dedicated app, then frontmost browser, then weak app (`:172-258`).
    - Per-key dedupe, with a reset after 10 idle evaluations (`:106, 176-188`). Suppression is 120 s after a dismiss and 15 s after recording (`:262-272`).
  - The browser URL comes from Accessibility `kAXDocumentAttribute` (`BrowserMeetingActivityCollector.swift:174-260`).
  - anarlog `plugins/detect/src/policy.rs:19-111` keeps a **deny-list by category**: dictation apps, IDEs, screen recorders, AI assistants and Raycast. Users can override it in both directions (`should_track_app`, `:133-140`).
    - It requires the mic to be held for **15 s** before prompting (`mic_usage_tracker.rs:8`), with a **10 min cooldown** per app after a prompt (`:9, 22-57`).
    - It respects Do Not Disturb (`dnd.rs`), although it reads DND by shelling out to `defaults`, which Deino should avoid.
  - anarlog `apps/desktop/src/stt/meeting-apps.ts`: the browser bundle-ID list (`:7-57`), a platform table (`:102-160`) and a URL-to-platform parser (`:384-470`: Zoom, Meet, Webex, Teams, Cal.com, Daily, Whereby, Jitsi, GoTo, Slack, Discord, X calls).
  - openwhispr `src/helpers/meetingProcessDetector.js:7-20` keeps a smaller table (Zoom, Teams, Webex, FaceTime) and uses `NSWorkspace` launch and terminate notifications. That is weaker, because a running app is not the same as an app in a call.

### Adopt for Deino (CAP-06)
```rust
/// One Core Audio client that is (or was) capturing input.
pub struct AudioProcess { pub pid: i32, pub bundle_id: BundleId, pub running_input: bool, pub running_output: bool }
pub enum ProcessEvent { InputStarted(AudioProcess), InputStopped(AudioProcess), Reconciled(Vec<AudioProcess>) }

pub trait ProcessAudioSource: Send + Sync {          // adapters/process_audio.rs (cidre)
    fn snapshot(&self) -> Result<Vec<AudioProcess>, DetectError>;
    fn subscribe(&self, tx: Sender<ProcessEvent>) -> Result<Subscription, DetectError>;
}
pub trait CameraSource: Send + Sync {                // adapters/camera.rs (CMIO FFI)
    fn subscribe(&self, tx: Sender<CameraEvent>) -> Result<Subscription, DetectError>;
}
pub trait FrontmostApp: Send + Sync { fn frontmost(&self) -> Option<BundleId>; } // NSWorkspace

pub enum MeetingApp { Zoom, Teams, Meet, Slack, FaceTime, WhatsApp, Webex, Discord, Browser(BrowserKind), Other(BundleId) }
pub enum Strength { Strong, Weak, Browser, Denied }
/// Pure: bundle id → app + strength. Table lives in detection/classify.rs.
pub fn classify(bundle: &BundleId) -> (MeetingApp, Strength);

/// Pure, Clock-driven: edges in → at most one Prompt out.
pub struct DetectionPolicy { /* dwell timers, cooldowns, ignore list, recording flag */ }
impl DetectionPolicy {
    pub fn on_signal(&mut self, s: Signal, now: Instant) -> Vec<DetectionAction>;
    pub fn tick(&mut self, now: Instant) -> Vec<DetectionAction>;
}
```
- **Signal source.**
  - Implement openwhispr's listener model in Rust with cidre: `add_prop_listener` on `System::OBJ` for `HW_PROCESS_OBJ_LIST`, plus one listener per process object for `PROCESS_IS_RUNNING_INPUT`.
  - The callbacks only send on a channel. Reconcile on the detection thread. Use anarlog's 1 s poll as a fallback when a listener fails to register.
  - Deino's current `system_detector.rs` reads `is_running_output` (audit), which is the wrong signal.
- **Exclusions:** Deino's own bundle ID and PID, `com.apple.CoreSpeech`, the anarlog deny categories (Dictation, IDE, ScreenRecording, AIAssistant, Raycast, GarageBand, the Sound settings extension), and the user's ignore list.
- **Relabelling:** `avconferenced` and `callservicesd` become FaceTime or iPhone Call (anarlog).
- **Dwell, to meet CAP-06's "within 3 s":**
  - **Strong** (a dedicated app with mic; camera with mic; or any mic use plus a calendar event within −15/+5 min): prompt after a 1.5 s dwell.
  - **Browser:** prompt after 1.5 s only if the browser is frontmost **and** (a calendar event is near or the meeting URL is known through Accessibility, once CAP-05 grants AX). Otherwise do not prompt.
  - **Weak** (Slack, WhatsApp): prompt only with corroboration (calendar, camera, or frontmost). Use the CAP-06 copy "Huddle detected" or "Call detected".
  - **Other unknown apps:** apply anarlog's 15 s dwell, because unknown apps holding the mic briefly are mostly false positives.
- **De-duplication:** 500 ms trailing-edge debounce on stop. A 10 min per-app cooldown after "Ignore", a permanent entry after "Always ignore", and suppression while recording (CAP-06).
- **Camera:** port muesli's CMIO observer as about 120 lines of `extern "C"` FFI to `CoreMediaIO.framework` (`CMIOObjectGetPropertyData`, `CMIOObjectAddPropertyListenerBlock` via `block2`, which is already a transitive dependency). First check whether an `objc2-core-media-io` crate is published at the pinned objc2 version; if it is, use it instead.

### Copy vs reimplement
- **anarlog `policy.rs` table, `list/macos.rs` relabel and self-filter, and the `mic_usage_tracker` cooldown (MIT):** copy and adapt with the notice.
- **openwhispr listener and reconcile logic and the CoreSpeech rule:** reimplement in Rust.
- **muesli detector priorities and camera observer:** reimplement in Rust. The app table may be reused, with attribution in a comment.

---

## 7. Auto-stop heuristics and recording health

### Best found: auto-stop
- **openwhispr `src/helpers/meetingAutoEndController.js:1-202` is the best design: tick-driven, timer-free and pure.**
  - **Ownership mode** (an external app held the mic): stop once the app has released the mic and **the system channel has stayed quiet** for `OWNERSHIP_CONFIRM_MS = 5 s` (`:5-12, 71-81`). Remote audio resuming restarts the window.
  - Holds shorter than `OWNERSHIP_MIN_ACTIVE_MS = 30 s` (Siri, a permission prompt) never arm ownership mode (`:13-16, 142-150`).
  - **Fallback mode:** both channels quiet for `SILENCE_WINDOW_MS = 60 s`, or `FAST_SILENCE_MS = 10 s` after the tracked app exits (`:1-4, 83-90`).
  - Sleep is detected by the tick gap (`:17-19, 41-59`).
- **anarlog `apps/desktop/src/stt/auto-stop.ts` and `detect-events.ts:159-330, 435-475`:**
  - Record the **trigger apps** of the session. When one of them stops, schedule a 5 s confirm (`AUTO_STOP_CONFIRM_DELAY_MS`).
  - At confirm time:
    - **re-snapshot** the mic-using apps;
    - check Accessibility for a still-active meeting UI;
    - if the network dropped, hold until the calendar end plus 10 min, or for 8 min (`:17-21, 45-97`);
    - for browsers, **ask** "Did your meeting end?" with a 5 s timeout instead of stopping (`shouldPromptBeforeAutoStopping`, `:33-43`);
    - skip apps whose mic signal is unreliable (KakaoTalk, `:23`).
  - Sleep stops immediately (`:466-470`).
- **muesli `MeetingAutoStopPolicy.swift:3-46`:** after a manual start, signal loss leads to no action. After a detected or calendar start, it leads to `warnOnly`. Muesli chose to prompt rather than auto-stop. `MeetingAutoStopTracker` (`:144-203`) uses a grace period with candidate matching by room URL or bundle.

### Best found: health
- **muesli `MeetingMicHealthTracker.swift:45-181`:**
  - `micCallbacksMissing`: system audio active for 3 s while mic callbacks are absent or more than 1 s stale.
  - `micAllZeroWhileSystemActive`: mic more than 99.9% zeros while system peaks above 0.01 for 3 s.
  - It keeps a bounded transition history of 32 entries (`:62-68, 97-130`).
- **muesli `MeetingSystemAudioWatchdog.swift:1-120`:**
  - "A flat IO callback stream is intentionally not treated as failure: healthy global process taps may quiesce while system output is silent" (`:16-25`).
  - Recovery runs in **episodes**: at most 4 attempts, a 15 s cooldown, and recovered after 2 healthy ticks (`:27-35`).
  - It has a mic "blindness bridge" after route events.
- **openwhispr `src/helpers/meetingSystemAudioWatchdog.js:1-60`:**
  - It does the opposite. It treats "no chunk for `STALL_MS = 6 s`" as a stall, but only because its helper "writes a chunk every period whatever the audio is".
  - It has `MAX_RESTARTS = 3`.
  - "Gone quiet for 180 s" is a warning only and never restarts capture.
- **Cap (reference only) `output_pipeline/core.rs:391-436` `PipelineHealthEvent`:** `FrameDropRateHigh`, `AudioGapDetected`, `SourceRestarting`/`Restarted`, `Stalled`, `MuxerCrashed`, `DiskSpaceLow`/`Exhausted`, `DeviceLost`, `EncoderRebuilt`, `SourceAudioReset` and `RecoveryFragmentCorrupt`.
  - `FrameDropTracker` uses a 5 s window with a 5% threshold (`macos_fragmented_m4s.rs:35-100`).
  - The health channel is bounded.
- **muesli transcript health:** `MeetingTranscriptHealthMonitor.swift:1-80` tracks successful, empty and failed chunk counts and speech-coverage ratio.

### Adopt for Deino
**CAP-07 `detection/autostop.rs`** is a pure state machine, a Rust port of openwhispr's controller extended with CAP-07's rules and anarlog's confirm step:
```rust
pub enum AutoStopInput {
    TriggerMicReleased { app: BundleId }, TriggerMicResumed { app: BundleId },
    TriggerAppExited { app: BundleId },
    Activity { mic_speech: bool, system_speech: bool }, // from VAD lanes, 1 Hz
    WillSleep, CalendarEnd { at: Instant }, UserKeepRecording,
}
pub enum AutoStopDecision { None, ShowEnding { stop_at: Instant, reason: StopReason }, Stop(StopReason) }
pub enum StopReason { MicReleased, NoSpeech, Sleep, CalendarEnded }

pub struct AutoStopPolicy { cfg: AutoStopConfig, state: AutoStopState }
impl AutoStopPolicy {
    pub fn on(&mut self, input: AutoStopInput, now: Instant) -> AutoStopDecision;
    pub fn tick(&mut self, now: Instant) -> AutoStopDecision; // also detects tick gaps (sleep)
}
pub struct AutoStopConfig {             // named consts with doc comments in the module
    pub mic_released: Duration,         // 20 s  (CAP-07)
    pub ending_toast: Duration,         // 15 s  (CAP-07)
    pub no_speech: Duration,            // 15 min (CAP-07)
    pub calendar_grace: Duration,       // 10 min after end, and no speech in last 2 min
    pub min_ownership: Duration,        // 30 s  (openwhispr)
    pub tick_gap: Duration,             // 5 s   (openwhispr)
}
```
- **Mic released:** the release must hold for 20 s with **no system-channel speech** (openwhispr's contradiction rule). Then show `ShowEnding` for 15 s (the toast with "Keep recording"), then `Stop`.
  - Before showing the toast, re-snapshot the process list (anarlog) so a flap or a screen-share rebuild of the input unit does not count.
  - Browser triggers always go through the toast (anarlog prompts for browsers).
- **Manual starts** never auto-stop on mic release; they get warn-only behaviour (muesli). The no-speech, sleep and calendar rules still apply.
- **Network-drop hold** (anarlog) is optional. Add it later if false stops show up in QA.
- **Unit tests** use a fake `Clock` for each stop path (CAP-07 acceptance): mic release with system quiet, mic flap, system resumes, no speech for 15 min, sleep, calendar end with recent speech versus without, tick gap after wake.

**CAP-10 `health/`** is also pure and driven by a 1 Hz tick. It reads atomics and counters and never touches the audio data:
```rust
pub struct HealthSnapshot {              // gathered once per second by the supervisor
    pub mic: TrackCounters, pub system: TrackCounters,   // callbacks, frames, dropped, peak, zero_ratio
    pub mic_vad_active_since: Option<Instant>, pub last_segment_at: Option<Instant>,
    pub stt_worker_alive: bool, pub free_disk: Bytes, pub writer_backlog: Bytes,
    pub last_route_event: Option<Instant>,
}
pub enum HealthEvent {
    TrackStalled { track: Track, for_: Duration },        // no callbacks ≥5 s (see system caveat)
    MicSilentWhileSystemActive,                           // muesli rule, 3 s
    FramesDropped { track: Track, pct: f32 },             // >0.5 % over 5 s window
    NoTranscript { for_: Duration },                      // VAD-active mic, no segment 30 s
    SttCrashed, DiskLow { free: Bytes }, DiskExhausted,
    StorageBackpressure { backlog: Bytes }, DeviceLost { track: Track },
    Recovering { track: Track, attempt: u8 }, Recovered { track: Track },
    BluetoothCallMode, SystemAudioPermissionSuspected,
}
pub struct HealthMonitor { /* last-seen counters, open episodes */ }
impl HealthMonitor { pub fn evaluate(&mut self, s: &HealthSnapshot, now: Instant) -> Vec<HealthEvent>; }
```
- **The system-track stall caveat.** The references disagree on whether a tap's IOProc keeps firing during silence (muesli says it can quiesce; openwhispr relies on it firing). Do not warn on frozen system callbacks alone. Warn only when:
  - (a) the aggregate reports `DEVICE_IS_ALIVE=false` or not running; or
  - (b) callbacks have been frozen for 5 s **and** a cheap rebuild attempt also produced no callbacks within 2 s; or
  - (c) any process is running output (from the detection snapshot) while the tap has delivered nothing for 5 s.

  Mic stalls of 5 s are always real, because the cpal input IOProc is clock-driven. CAP-10's acceptance ("unplug mic … warning within 10 s") is met by `DeviceDied` plus the 5 s stall rule.
- **Recovery episodes** (muesli): at most 4 attempts per episode, a 15 s cooldown, closed after 2 healthy ticks, with retries paused for 3 s after a route event.
- **Surfacing:** `HealthEvent`s go to a Tauri event (`recording://health`) and to the tray and pill state (orange). Notifications fire only on the first transition into degraded. The latest summary is stored in `recording.json`.

### Copy vs reimplement
- **openwhispr auto-end controller:** reimplement in Rust. It is about 150 lines of logic, so cite it in the module doc.
- **muesli health trackers and episode policy:** reimplement.
- **Cap health enum:** principle only. Deino's enum is its own.

---

## 8. Rust crates for Core Audio and ObjC interop

| Crate | Used by | Covers | Recommendation for Deino |
|---|---|---|---|
| `cidre` | Deino (git `a9587fa`), anarlog (0.15 with `core_audio, av, ax, cm, io_kit, io_pm`), Cap (fork `bf84b67`) | Process taps (`TapDesc`, `TapGuard` with Drop), aggregates (Drop destroys), `System::processes()`, `Process::{is_running_input, bundle_id}`, `HW_PROCESS_OBJ_LIST`, `add_prop_listener`/`_block`, `DEVICE_IS_ALIVE`, `DEVICE_PROCESSOR_OVERLOAD`, `TAP_FORMAT`, `NOMINAL_SAMPLE_RATE`, `av` (AVAssetWriter), `sc` (ScreenCaptureKit), `mach::abs_time` | **Keep it as the single Core Audio, AV and ScreenCaptureKit binding.** Pin the rev in an ADR. Wrap every use in `adapters/`, so a cidre API change touches one module. |
| `objc2` + `objc2-app-kit` / `objc2-foundation` / `block2` | anarlog (`sleep.rs`, `list/macos.rs`), Handy (`tauri-nspanel`) | `NSWorkspace` sleep, wake and frontmost app; `NSRunningApplication`; notification blocks | **Use** for `power.rs`, `FrontmostApp` and the pill (§3C names the objc2 family). |
| `objc2-core-audio` 0.3.2 | none of the references for taps | Raw `AudioHardwareCreateProcessTap`, `CATapDescription`, `kAudioProcessProperty*` (generated `AudioHardware.rs:1760-1861`) | **Do not add it now.** It duplicates cidre. Keep it as the escape hatch if cidre lacks something. |
| `coreaudio-rs` / `coreaudio-sys` | cpal internally | AudioUnit wrappers | Not directly. Use it only through cpal. |
| `screencapturekit` crate | none of the references (Cap uses cidre `sc` through `scap-screencapturekit`) | ScreenCaptureKit | **Do not add it.** Use `cidre::sc` for CAP-09 and the ScreenCaptureKit audio fallback. |
| `cpal` | Deino (git rev), anarlog 0.17, Handy 0.16, Cap | Mic capture, with host timestamps | Keep it for the mic. Never request exclusive access (§3C). |
| `rtrb` 0.4 | Handy | Wait-free SPSC, `write_chunk_uninit` | **Adopt** for the RT rings (§3C names it). |
| `ringbuf` 0.4.8 | Deino, anarlog | SPSC | Remove from audio capture once rtrb is in place. |
| `rubato` | Deino 0.15, anarlog 1.0.1 (+`audioadapter-buffers`), Handy 0.16 | Resampling; `Async` resampler supports relative ratio changes for drift | Keep it. Use the async resampler with `set_resample_ratio_relative` for drift. Upgrade to 1.x in its own PR. |
| CoreMediaIO | muesli (Swift) | Camera "in use" | No crate in the references. Use a small FFI module (or `objc2-core-media-io` if it is published at the pinned objc2 version). |
| `tauri-nspanel` | Handy (`overlay.rs:450-485`) | Non-activating `NSPanel` for the pill | **Adopt for CAP-10:** `PanelLevel::Status`, `nonactivating_panel()`, `focusable(false)`, `can_join_all_spaces + full_screen_auxiliary`. This is what "never steals focus" needs. |

---

## 9. Target audio architecture for Deino

### 9.1 Module layout under `frontend/src-tauri/src/audio/` (each file 300 lines or fewer, each function 40 lines or fewer)
```
audio/
  mod.rs                    //! map of the audio subsystem; re-exports
  types.rs                  Track{Mic,System}, SampleRate, Frames, HostTime, Nanos, BundleId, DeviceUid (newtypes)
  error.rs                  CaptureError, EncodeError, DeviceError (thiserror)
  capture/
    mod.rs                  trait CaptureSource, trait SourceFactory, CaptureHandle
    rt_sink.rs              RtSink, RtCounters, BlockStamp (the only code that runs on RT threads)
    mic_cpal.rs             MicSource: cpal input → RtSink (Handy pattern)
    system_tap/
      mod.rs                SystemTapSource: CaptureSource
      tap.rs                tap + aggregate creation, naming, stable/fallback UID, readiness wait
      io_proc.rs            extern "C" IOProc → RtSink (format conversion)
      stale.rs              launch sweep of public deino-/meetily- tap devices
    sck_audio.rs            ScreenCaptureKit audio fallback backend (later; §3C)
    fake.rs                 #[cfg(test)] FakeSource: scripted tones, gaps, rate changes, deaths
  timeline/
    clock.rs                TrackClock: host time → 48 kHz timeline, Placement, pause offsets
    drift.rs                DriftEstimator (EMA ppm)
    resample.rs             Resampler wrapper: preallocated rubato Async, rebuild on rate change
  conditioner.rs            per-track worker: drain ring → place → resample → fan-out
  fanout.rs                 AudioBlock routing: writer (bounded), STT (drop-oldest), level atomics
  writer/
    mod.rs                  RecordingWriter (writer thread), trait TrackEncoder, trait EncoderFactory
    ffmpeg_fmp4.rs          FfmpegFragmentedAac
    pcm.rs                  PcmEncoder (tests, fixtures)
    persister.rs            flush/fsync cadence, in-memory backlog, disk checks → StorageHealth
    manifest.rs             RecordingManifest (serde), atomic write
    recovery.rs             launch scan: probe .part, truncate, rename, report
    finalize.rs             pipeline stage `finalize_audio`: close segments, concat, mix → audio.mp4
  devices/
    monitor.rs              Core Audio listeners → DeviceEvent (callbacks only send)
    route.rs                headphone/Bluetooth/HFP classification, preferred mic choice
    power.rs                NSWorkspace sleep/wake → PowerEvent
  health/
    mod.rs                  HealthMonitor (pure), HealthEvent
    snapshot.rs             HealthSnapshot assembly from counters (supervisor side)
  session/
    state.rs                RecordingState enum + transitions (Idle→Starting→Recording⇄Paused→Stopping→Closed; Recovering)
    supervisor.rs           event loop: DeviceEvent | PowerEvent | HealthEvent | Command → actions
    service.rs              start/stop/pause/switch-mic/toggle-system (no Tauri types)
    commands.rs             thin Tauri commands (§3A layer 1)
  permissions.rs            CapturePermissions trait + macOS impl (existing file, rewritten)
detection/                  (new top-level, CAP-06/07)
  mod.rs
  process_audio.rs          ProcessAudioSource impl (cidre listeners + reconcile)
  camera.rs                 CameraSource impl (CMIO FFI)
  classify.rs               bundle-id table → (MeetingApp, Strength)
  policy.rs                 DetectionPolicy (pure): dwell, cooldown, ignore list, suppression
  autostop.rs               AutoStopPolicy (pure)
  service.rs                wires sources + policy + calendar lookup → prompt window events
```
The following legacy files are deleted or absorbed: `pipeline.rs` (mixer and VAD move to the conditioner and to CAP-02's lanes), `stream.rs`, `incremental_saver.rs`, `recording_saver*.rs`, `core-old.rs`, `system_detector.rs`, `device_monitor.rs` and `ffmpeg_mixer.rs`. Each card that touches one of them splits it (§3A, "legacy files get split").

### 9.2 Threads, channels and data flow
```
 CoreAudio HAL RT thread (mic, cpal)        CoreAudio HAL RT thread (aggregate IOProc)
   └─ RtSink::push_interleaved ─┐              └─ RtSink::push_interleaved ─┐
        rtrb<f32> 2 s + rtrb<BlockStamp>            rtrb<f32> 2 s + rtrb<BlockStamp>
        AtomicU64 counters                          AtomicU64 counters
                                │ (polled every 10 ms, no wakeups from RT)  │
                    ┌───────────▼───────────┐                   ┌───────────▼───────────┐
                    │ conditioner-mic thread │                   │ conditioner-sys thread │
                    │ TrackClock.place       │                   │ (same)                 │
                    │ gap→zeros / overlap→cut│                   │                        │
                    │ rubato → 48 kHz, 16 kHz│                   │                        │
                    └──┬────────┬────────┬───┘                   └──┬────────┬────────┬───┘
       crossbeam bounded│  bounded│drop-  │AtomicU32 RMS              │        │        │
        (~8 s, never    │  oldest │       ▼                           │        │        ▼
         silent drop)   │        │   level meter → pill (30 fps)       │        │   level meter
                        ▼        ▼                                     ▼        ▼
              ┌──────────────────────┐   ┌──────────────┐    ┌──────────────┐
              │ writer thread        │   │ STT lane: Me │    │ STT lane: Them│   (CAP-02: VAD+STT per channel)
              │ TrackEncoder×2       │   │ VAD → STT    │    │ VAD → STT     │
              │ (ffmpeg fMP4 AAC)    │   └──────┬───────┘    └──────┬────────┘
              │ backlog ≤64 MiB      │          │ segments{channel,start_ms,end_ms}
              └─────────┬────────────┘          ▼
                        │ PersistOp     transcript_segments (DB, FND-05/07)
              ┌─────────▼────────────┐
              │ persister thread     │  flush 1 s · fsync 5 s · manifest 5 s · disk check 5 s
              └─────────┬────────────┘
                        ▼
   meetings/<id>/recording.json, audio/{mic,system}.NNN.m4a(.part)

 device-listener thread (Core Audio prop listeners: send only) ─┐
 power thread (NSWorkspace sleep/wake)                          ├─► supervisor (tokio task, 1 Hz tick)
 detection thread (process objects + CMIO; send only)           │    ├─ RecordingState transitions
 RtCounters / writer stats (read-only)                          ┘    ├─ restart a CaptureSource (per track)
                                                                     ├─ HealthMonitor.evaluate → events
                                                                     └─ AutoStopPolicy.tick → toast/stop
```
- **RT threads:** no allocation, no locks, no logging, no syscalls. They only write the ring and atomics.
- **Conditioners:** one `std::thread` per track. All buffers are preallocated at source start and rebuilt only on a rate change, never per block.
- **Ownership:** each `CaptureSource` owns its HAL objects (RAII). The supervisor owns the sources, and can drop and recreate one track without touching the other track's conditioner, so the timeline stays continuous.
- **Mid-recording mic switch or system toggle (CAP-10):** the supervisor swaps the mic `CaptureSource` and leaves the conditioner and the clock alone. Disabling system audio stops the tap, and the clock fills the system track with silence.

### 9.3 Core traits (dependency inversion; wired in `app_state.rs`)
```rust
pub trait CaptureSource: Send {
    fn track(&self) -> Track;
    fn native_rate(&self) -> SampleRate;
    fn start(&mut self, sink: RtSink) -> Result<(), CaptureError>;
    fn stop(&mut self);
}
pub trait SourceFactory: Send + Sync {
    fn mic(&self, choice: &MicChoice) -> Result<Box<dyn CaptureSource>, CaptureError>;
    fn system(&self) -> Result<Box<dyn CaptureSource>, CaptureError>;
}
pub trait EncoderFactory: Send + Sync { /* §5 */ }
pub trait DeviceEvents: Send + Sync { /* §3 */ }
pub trait PowerEvents: Send + Sync { fn subscribe(&self, tx: Sender<PowerEvent>) -> Result<Subscription, DeviceError>; }
pub trait DiskSpace: Send + Sync { fn free_bytes(&self, path: &Path) -> std::io::Result<Bytes>; }
pub trait Clock: Send + Sync { fn now(&self) -> Instant; }           // shared FND clock
pub trait HostClock: Send + Sync { fn host_time(&self) -> HostTime; } // mach_absolute_time
pub trait ProcessAudioSource, CameraSource, FrontmostApp            // §6
pub trait CapturePermissions                                          // §1
```
**Fakes for tests:**
- `FakeSource`: scripted blocks with stamps, gaps, death and rate change.
- `PcmEncoder` writing into a temp directory.
- `FakeDisk`, `ManualClock`, `FakeProcessAudio`, `FakeCamera`.

**Contract tests** run every `TrackEncoder` against "truncate at a random byte, then decode, and the playable duration is at least the written duration minus 2 s".

### 9.4 Files written (per meeting)

| File | Writer | Cadence | Crash state |
|---|---|---|---|
| `recording.json` | persister | atomic, every 5 s and on events | always valid JSON, at most 5 s stale |
| `audio/mic.NNN.m4a.part` | writer, persister | flush 1 s, fsync 5 s | playable up to the last fragment |
| `audio/system.NNN.m4a.part` | writer, persister | flush 1 s, fsync 5 s | playable up to the last fragment |
| `mic.m4a`, `system.m4a`, `audio.mp4` | `finalize_audio` stage | once | written to temp, then renamed |
| `transcript_segments` rows | STT lanes (FND-05) | per finalized segment | committed rows survive |

### 9.5 How this meets §3A and §3C
- **§3A, layers and dependencies:** commands → service → adapters (cidre, cpal, ffmpeg, NSWorkspace, CMIO), with every adapter behind a trait. The state machines (`RecordingState`, `DetectionPolicy`, `AutoStopPolicy`, `HealthMonitor`, `TrackClock`) are pure and unit-tested with `ManualClock`.
- **§3A, types:** newtypes for units and IDs; one `thiserror` enum per module; named constants with doc comments for every threshold (20 s, 15 s, 15 min, 5 s, 2 GB, 64 MiB, ±20 ms and so on). Logs carry IDs and counts only.
- **§3C:**
  - process tap and aggregate (macOS 14.2+), with ScreenCaptureKit audio as a fallback backend only;
  - cpal mic on the chosen device or the default, following the default;
  - Bluetooth HFP warning and built-in-mic preference;
  - Core Audio process objects for detection;
  - RT-safe callbacks with rtrb;
  - native rate resampled once to 48 kHz and 16 kHz;
  - separate tracks end to end;
  - sleep, wake, device death, rate change and tap invalidation handled with health events;
  - stale tap cleanup;
  - permission explainers and deep links;
  - objc2 and cidre only, with no AppleScript.

---

## 10. Recommendations by card

### FND-05 · Rust-owned live persistence and multitrack audio files
- **Read first:**
  - anarlog `crates/listener-core/src/actors/recorder/chunks.rs:17-30, 82-125, 229-560, 563-787, 955-1003` and `recorder/mod.rs:48, 120-195`;
  - Cap `crates/recording/src/fragmentation/mod.rs:11-29`, `fragmentation/manifest.rs:1-58`, `recovery.rs:285, 849` (principles only);
  - muesli `MeetingRecordingWriter.swift:7-21` (recovery reference).
- **Recommendations:**
  - Introduce `writer/` with `TrackEncoder`, a writer thread and a persister thread. Default to `FfmpegFragmentedAac` (1 s fragments) per track. Flush every 1 s and `fsync` plus rewrite the manifest every 5 s. Together these meet the "about 5 s loss" criterion.
  - Write only `mic` and `system` live. Derive `audio.mp4` in `finalize_audio` by summing the sample-aligned tracks. Record this in an ADR as the way "checkpoint recovery for all three" is satisfied.
  - Put the sample-index-is-time rule in `timeline/clock.rs` first (host-time stamps, gap and overlap handling). The "time-aligned within 50 ms" check then becomes a unit test over `FakeSource` scripts, plus one integration test that compares cross-correlation of a click fixture between the files.
  - Recovery at launch: probe `.part` files, truncate the manifest to the playable length, rename, set the meeting to `processing` and enqueue FND-08. Delete the IndexedDB transcript recovery once this ships.
  - Keep a bounded backlog of 64 MiB or less, with `StorageHealth` events (anarlog). Disk below 256 MiB stops the recording cleanly. Never block the conditioner.
  - Acceptance test: a script runs a recording against fixture audio, sends `kill -9` at a random time, relaunches, and asserts that the meeting exists, that the audio and segment loss is 5 s or less, and that both `.m4a` files decode.

### CAP-02 · Me/Them channel separation
- **Read first:**
  - anarlog `crates/audio/src/lib.rs` (`CaptureFrame`), `crates/audio-actual/src/capture/joiner.rs:5-58`, `capture/stream.rs:25-35, 137-210, 212-407`;
  - `crates/audio-sync/src/probe.rs:52-110`, `drift.rs:1-39`;
  - `crates/listener-core/src/actors/source/stream.rs:80-125` (AEC on or off by route);
  - Cap `crates/timestamp/src/master_clock.rs:260-340` (principle).
- **Recommendations:**
  - The fan-out gives each conditioner its own STT lane: VAD then STT, `channel` tagged at the source. Delete the 600 ms `AudioMixerRingBuffer`, so no mixing happens before STT.
  - Segment timestamps come from the shared timeline (`TrackClock`). Mic and system segments then interleave correctly without any pairing logic.
  - Bleed suppression runs after the STT lanes, on segments: same window, normalized Levenshtein ratio above 0.6. As a second signal, use a cheap mic-versus-system cross-correlation over lags of 0 to 600 ms (the anarlog `SyncProbe` idea). Skip it when `route.rs` says headphones-only, or when the user has set "I use headphones".
  - Defer DTLN-aec (anarlog `crates/aec`, ONNX). It probably breaks the 1.6× CPU budget. Revisit only if the duplicated-line target (under 5%) is missed.
  - Measure CPU with both lanes on the 10-minute fixture. Run VAD at 16 kHz on 30 ms frames per lane, and only send STT the VAD-positive spans.

### CAP-06 · Meeting detection and auto-start
- **Read first:**
  - openwhispr `resources/macos-mic-listener.swift:219-330`;
  - anarlog `crates/detect/src/list/macos.rs:11-151`, `list/mod.rs:24-92`, `mic/macos/state.rs:26-84`, `plugins/detect/src/policy.rs:19-140`, `mic_usage_tracker.rs:8-102`, `apps/desktop/src/stt/meeting-apps.ts:7-160, 384-470`;
  - muesli `MeetingDetector.swift:62-348`, `CameraActivityMonitor.swift:74-253`, `AudioProcessAttributionCollector.swift:33-197`.
- **Recommendations:**
  - Rewrite `system_detector.rs` as `detection/process_audio.rs`, with listeners on `HW_PROCESS_OBJ_LIST` and per-object `PROCESS_IS_RUNNING_INPUT` (cidre). Fall back to a 1 s poll. Use `is_running_input`, not `is_running_output`.
  - Add `detection/camera.rs` (CMIO FFI). Camera only strengthens mic use; it never prompts on its own.
  - `classify.rs`: muesli's strong and weak table, anarlog's browser IDs and URL parser, anarlog's deny categories, the Apple call-daemon relabel, and the CoreSpeech and self exclusions.
  - `DetectionPolicy` (pure): 1.5 s dwell for strong signals, to meet the 3 s target; corroboration for browser and weak apps; 15 s dwell for unknown apps; 500 ms stop debounce; 10 min cooldown after "Ignore"; permanent "Always ignore"; suppressed while recording.
  - The prompt window should be a non-activating `NSPanel` (Handy's `overlay.rs:450-485` pattern). Look up a calendar event with start in [now − 15 min, now + 5 min]. Run the auto-start countdown in Rust, with "Don't record" cancelling it.
  - Tests: table-driven `classify` tests, plus `DetectionPolicy` scenarios with `ManualClock`: a Zoom join, a browser frontmost with a calendar event, Slack in the background, Photo Booth (camera only), Siri (a 2 s mic hold), and an ignored app.

### CAP-07 · Auto-stop and meeting boundaries
- **Read first:**
  - openwhispr `src/helpers/meetingAutoEndController.js:1-202` (the core model);
  - anarlog `apps/desktop/src/stt/auto-stop.ts:17-97`, `detect-events.ts:159-330, 435-475`;
  - muesli `MeetingAutoStopPolicy.swift:3-46, 144-230`;
  - anarlog `crates/detect/src/sleep.rs:1-107`.
- **Recommendations:**
  - Port openwhispr's controller to `detection/autostop.rs` as `AutoStopPolicy`, with `on(input, now)` and `tick(now)`. It has no timers, so every stop path can be unit-tested with `ManualClock` (acceptance).
  - **Mic released:** the release must hold for 20 s with no system speech. Then re-snapshot the process list (anarlog's confirm), show the 15 s "Ending…" toast with "Keep recording", then stop. Holds shorter than 30 s never arm ownership mode.
  - **No speech for 15 min**, using both VAD lanes. **Sleep:** stop on `WillSleep`. The tick-gap fallback resets windows after wake. **Calendar:** end plus 10 min, and no speech in the last 2 min.
  - **Manual starts:** warn only on mic release (muesli). Browser triggers always get the toast (anarlog).
  - **Back-to-back meetings and Resume:** the writer opens segment `NNN+1`, and the manifest records segment boundaries in timeline frames. A split creates a new meeting whose `t0` is the split frame. Resume appends a new segment on the same timeline, offset by the pause.

### CAP-09 · Screen recording, shared-screen snapshots, screen context
- **Read first:**
  - Cap `crates/scap-screencapturekit/src/{config.rs,capture.rs}` and `crates/recording/src/sources/screen_capture/macos.rs:240-260, 960-980` (reference only; the cidre `sc` usage pattern);
  - Cap `output_pipeline/macos_fragmented_m4s.rs:35-100, 150-240` (2 s fragments, drop-rate health);
  - anarlog `crates/detect/src/screen.rs`;
  - muesli `CoreAudioSystemRecorder.swift:77-83` (why it chose the Core Audio tap over ScreenCaptureKit for audio).
- **Recommendations:**
  - Use `cidre::sc` and do not add the `screencapturekit` crate. Request Screen Recording only when CAP-09 is enabled (§3C).
  - **Video:** H.264 at 1 fps in fragmented MP4 (2 s fragments, Cap's principle). Same manifest and persister as the audio. Video frames use host-time timestamps on the **same `t0`** as the audio, so snapshots line up with segments.
  - **Snapshots:** every 10 s, perceptual-hash the window frame and keep frames that changed beyond the threshold. Write PNGs through the persister (atomic rename).
  - Expose ScreenCaptureKit system audio as the `sck_audio.rs` fallback `CaptureSource` (§3C), selected only when the tap fails to be created.
  - Health: a `FrameDropRateHigh` analogue (more than 5% over 5 s) and a "window hidden: paused" state.

### CAP-10 · Record panel, recording health, menu bar
- **Read first:**
  - Handy `src-tauri/src/overlay.rs:450-485` (non-activating NSPanel) and `:730-767` (level emit throttle, `emit_to`);
  - muesli `MeetingMicHealthTracker.swift:45-181`, `MeetingSystemAudioWatchdog.swift:1-120`, `MeetingAudioRecoveryDeadlines.swift:6-71`, `CoreAudioSystemRecorder.swift:62-75`;
  - openwhispr `src/helpers/meetingSystemAudioWatchdog.js:1-60`;
  - Cap `output_pipeline/core.rs:391-436` (principle).
- **Recommendations:**
  - Build the pill with `tauri-nspanel`: `nonactivating_panel`, `focusable(false)`, status level, all Spaces. This meets "never steals focus". Feed the level meter from the conditioners' `AtomicU32` RMS at 30 fps or less, with `emit_to("record_panel", …)` only.
  - Implement `health/` as a pure `HealthMonitor` over a 1 Hz `HealthSnapshot`. It covers: track stall 5 s (with the system caveat in §7), mic silent while system active 3 s, frames dropped over 0.5%, VAD-active mic without a segment for 30 s, STT worker dead, disk under 2 GB, writer backlog, device lost, Bluetooth call mode, and permission suspected.
  - **Recovery episodes:** retry backoff `[2 s, 5 s]`, then at most 4 attempts with a 15 s cooldown, and a 3 s settle after route events (muesli). Each attempt emits `Recovering` and `Recovered`, and the pill turns orange while degraded.
  - **Mic switch and system toggle mid-recording:** the supervisor swaps one `CaptureSource`. The timeline and writer continue, and the clock inserts silence for the swap gap. Acceptance ("unplug mic → warning within 10 s") comes from the `DeviceDied` listener plus the stall rule.
  - **Menu bar:** subscribe to `recording://health` and the elapsed time. Use the "Xm left" data from ORG-02.

---

## 11. Risks and open questions
- **Does fragmented MP4 survive `kill -9` with the ffmpeg flags given?** Verify before building on it: truncate files and run ffprobe. Symphonia's support for `moof` must also be tested. If either fails, switch the default to `AvAssetWriterAac` with `movieFragmentInterval`.
- **Tap callbacks during silence.** The evidence conflicts (muesli says taps can quiesce; openwhispr assumes a chunk per period). Measure on macOS 14.x, 15.x and 26.x with nothing playing, then tune the system-stall rule.
- **Private aggregate leftovers after a crash.** The stable-plus-fallback UID handles them. Measure whether coreaudiod reclaims private aggregates when the owning process dies; muesli's comments say "phantoms" can persist until coreaudiod restarts.
- **The TCC preflight is private SPI.** Keep it behind a feature flag. The non-SPI fallback is an all-zeros heuristic.
- **The 1.5 s dwell for strong signals** trades false positives against the 3 s target. Use QA telemetry, counted locally only, to tune it.
