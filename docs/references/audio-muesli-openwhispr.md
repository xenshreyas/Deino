# Muesli and OpenWhispr: audio, detection and diarization patterns for Deino

- **Muesli:** `~/Documents/GitHub/deino-references/muesli/native/MuesliNative/Sources/MuesliNativeApp/` (Swift, MIT). Its vendored Qwen3ASR is Apache-2.0. Its SPM dependencies (FluidAudio Apache, WhisperKit, dtln-aec-coreml, the LocalVQE model) have their own licences.
- **OpenWhispr:** `~/Documents/GitHub/deino-references/openwhispr/` (Electron plus Swift/C++ helpers, MIT). Its mediaremote-adapter is BSD-3. Its AEC helper builds ChromiumOS webrtc-apm (BSD-3) and abseil (Apache).

Both repos are Swift or JS, so Deino re-implements these designs in Rust; the licences allow porting the logic.

## Muesli

### System audio capture
`CoreAudioSystemRecorder.swift`
- **Tap:** a process tap, with ScreenCaptureKit as fallback.
  - `CATapDescription(stereoGlobalTapButExcludeProcesses: [ownProcessObject])`, where the own process comes from `kAudioHardwarePropertyTranslatePIDToProcessObject`. `isPrivate=true`, `muteBehavior=.unmuted` (`:291-385, 700-714`).
  - The private aggregate device has tap drift compensation and TapAutoStart.
  - **The tap list must be UID dictionaries; passing `CATapDescription` objects crashes CoreAudio** (`:312-314`).
- **Stable aggregate UIDs:** use a *stable* UID (e.g. `com.xenshreyas.deino.system-audio-tap`) plus one deterministic fallback. Per-run UUIDs leak HAL settings, because private aggregates can't be enumerated (`:316-342`). There is also a stale sweep by name (`:770-811`).
- **Route changes:** only timestamp them. A global tap survived AirPods connects in their live tests (`:879-889`), so rebuild only on explicit failure, with a 1.5 s settle debounce and 2 s → 5 s backoff (`:62-142, 856-952`).
  - ⚠️ OpenWhispr's comment says the tap never follows route changes. **Deino must test this itself.**
- **Permission:** probe-tap flow (`:680-759`). Create and destroy a probe tap to check access. On first use, poll `AudioHardwareCreateProcessTap` every 300 ms for up to 12 s.

### Microphone and echo
- **Bluetooth:** prefer the built-in mic so AirPods don't drop to HFP (`AudioRouteController.swift:521-546`).
- **Make-before-break mic handoff** with a 5 s deadline scheduled before start. Health recovery swaps to the other backend (`MeetingMicRecording.swift:436-537`).
- **Neural AEC** (`MeetingNeuralAec.swift:54-177, 564-641`):
  - LocalVQE GGUF, with DTLN-aec CoreML as fallback.
  - It keeps a position-aligned system history and waits up to 3 s for the reference.
  - Delay is estimated by RMS-envelope cosine similarity over 0–800 ms in an 8 s window, re-estimated every 2 s.
- **Run VAD on the AEC-cleaned mic**, not the raw mic, so bleed doesn't create false "You" chunks (`MeetingSession.swift:1171-1240`).
- **Generation counters** drop callbacks from torn-down graphs (`CoreAudioSystemRecorder.swift:390-408`).
- ⚠️ The IOProc is not real-time safe (it allocates and locks). Don't copy that part.

### Meeting detection (the best of all references)
- **Per process:** `kAudioHardwarePropertyProcessObjectList` plus `IsRunningInput`, `IsRunningOutput`, `BundleID` and `Devices` (`AudioProcessAttributionCollector.swift:34-196`).
- **Mic device:** `DeviceIsRunningSomewhere` on the default input, plus a mute/zero-volume probe (`MicrophoneActivityObserver.swift:100-289`).
- **Camera:** `kCMIODevicePropertyDeviceIsRunningSomewhere` listeners on every CMIO device (`CameraActivityMonitor.swift:141-199`).
- **Bundle-ID table** (`MeetingCandidateResolver.swift:246-274`):
  - Meeting apps: `us.zoom.xos`, `us.zoom.ZoomPhone`, `com.apple.FaceTime`, `com.microsoft.teams2`, `com.microsoft.teams`, `com.tinyspeck.slackmacgap`, `com.webex.meetingmanager`, `com.cisco.webexmeetingsapp`, `net.whatsapp.WhatsApp`.
  - **Weak apps** that need full duplex (input *and* output) before they count: Slack, WhatsApp.
  - Browsers: Chrome, Brave, Arc (`company.thebrowser.Browser`), Edge, Safari. Helper processes are matched by bundle-ID prefix.
- **Browser URLs** (`BrowserMeetingActivityCollector.swift:173-317`):
  - Accessibility `kAXDocumentAttribute` on the focused, main and all windows.
  - Fallback: ScriptingBridge active tab, targeted by PID (so a quit browser is never relaunched), with a 1.8 s deadline.
- **URL normalizer:** Meet `xxx-xxxx-xxx`, Zoom `/j/<id>`, Teams, Webex, FaceTime, Slack huddle (`MeetingCandidateResolver.swift:155-238`).
- **Resolver priority** (`:286-453`):
  1. browser URL + attributed input process
  2. focused browser URL
  3. calendar event + audio app or browser
  4. browser audio
  5. dedicated app with full duplex
  6. foreground app with mic + camera
- **Timing** (`MeetingSignalRefreshPolicy.swift:146-156`, `MeetingPromptStateMachine.swift:49, 198-202`):
  - 0.5 s debounce.
  - Fallback polling every 120 s when idle, every 3 s when suspicious.
  - A candidate must be stable for **3 s** before the prompt. Browser auto-dismiss cooldown is 120 s.

### Auto-stop and health
- **Auto-stop warns, it doesn't stop.**
  - Grace period: **20 s**.
  - The warning is suppressed if a transcript appeared in the last 45 s.
  - Prompt: "Meeting signal lost", auto-dismissed after 30 s.
  - Source: `MeetingAutoStopPolicy.swift:181-229`, `MuesliController.swift:577-578, 9404-9454`.
- **Mic health:** the mic is degraded if system peak is above 0.01 while the mic is all zeros, or callbacks are missing for 3 s. Callbacks older than 1 s count as stale (`MeetingMicHealthTracker.swift:62-130`).
- **System-audio watchdog:** needs 2 healthy ticks to clear, 15 s cooldown, at most 4 attempts (`MeetingSystemAudioWatchdog.swift`).
- **Finite verification deadlines after a route change:** 3, 8, 20, 40 and 60 s, never idle polling (`MeetingAudioRecoveryDeadlines.swift`).
- **Transcript coverage audit** against VAD speech: below 0.55 coverage triggers a selective repair, below 0.60 a full re-transcription (`MeetingTranscriptHealthMonitor.swift:105-113`). **Great idea for CAP-10.**
- ⚠️ **Crash safety is weak** (a WAV header patched only on stop). Don't copy it.

### Other
- **Retranscription:** 5 s windows with 0.4 s overlap, and word-level overlap de-duplication of up to 8 words (`MeetingRecordingTranscriber.swift:138-185`).
- **Diarization:** offline, system track only. Speakers are assigned by maximum time overlap. There are no voiceprints.
- **GPU crash workaround:** M1 + macOS 15.1 forces `.cpuAndNeuralEngine`.

## OpenWhispr

### System audio
`resources/macos-audio-tap.swift`
- A process tap in a helper process: mono, private, fresh UUID per run, `TapAutoStart=false`.
  - It polls `kAudioDevicePropertyDeviceIsAlive` for up to 2 s before starting.
  - Output: Int16 at 24 kHz in 100 ms chunks.
- **Device events:** listeners on the default output and the aggregate's `IsAlive`, coalesced over 300 ms into `device_invalidated`.
- **Watchdog restart:** on that event or a 6 s chunk stall, at most 3 times (`meetingSystemAudioWatchdog.js`).
- **Timeline repair:** silence is inserted after a restart (`meetingAudioTimeline.js`).
- **Sleep detection:** a tick gap over 5 s means the machine slept, and pre-gap evidence is discarded.

### Echo and bleed
- **WebRTC AEC3 config:** 24 kHz input upsampled to 48 kHz, 10 ms frames, high-pass filter, moderate noise suppression (`native/meeting-aec-helper/src/aec_processor.cc:16-150`). Mic chunks are held 320 ms for reference alignment.
- **Echo-leak detector** (`meetingEchoLeakDetector.js:7-213`):
  - Lagged normalized correlation up to 500 ms, in 5 ms steps.
  - Strong bleed = correlation ≥ 0.75, residual ≤ 0.32, explained ≥ 0.65.
  - Partial-bleed detection over a 600 ms window.
- **Mic gate** (`meetingMicGate.js`): silence below RMS 0.0015 / peak 0.05 is dropped. A quiet mic (RMS < 0.018) while the system channel is speaking counts as "bleed_floor".
- **Text-level de-duplication:** hold risky mic finals for 3 s and drop them only on a *text* match with system audio (`meetingMicHoldback.js`). Their field logs show real local speech can correlate at 0.73–0.81 during double-talk, so audio correlation alone can't be trusted.

### Diarization and voiceprints (closest to Deino's planned stack)
- **Offline:** `sherpa-onnx-diarize` with pyannote-seg-3.0 and **3D-Speaker CAM++**: `cluster-threshold=0.55`, `min-duration-on=0.2`, `min-duration-off=0.5` (`src/helpers/diarization.js:54-64, 286-318`).
- **Transcript merge:** overlap first, nearest segment as fallback. The mic is "you"; a mic-only session with a single cluster stays "you" (`diarization.js:446-531`).
- **Live identification** (`liveSpeakerIdentifier.js`):
  - CAM++ 512-dimension embeddings from 1.5–8 s segments (`speakerEmbeddings.js`).
  - `MATCH_THRESHOLD=0.65`, `MATCH_MARGIN=0.03`.
  - Running-mean centroids; overflow never pollutes a centroid.
  - **IDs are never reused.**
  - Re-clustering never merges speakers with different confirmed identities.
- **Schema** (`database.js:968-1000`): `speaker_profiles(embedding BLOB, sample_count)`, `speaker_mappings`, `note_speaker_embeddings`.
- **Voiceprint update:** **EMA `0.3*incoming + 0.7*stored`**, keyed by id, then email, then name (`database.js:5877-5932`).

### Detection and auto-end
- **Mic listener helper** (`resources/macos-mic-listener.swift:224-644`):
  - PID-scoped `ProcessObjectList` + `IsRunningInput` listeners, **ignoring `com.apple.CoreSpeech`**.
  - Falls back to per-device `IsRunningSomewhere`; 5 s heartbeat reconcile.
- **Prompt rule:** sustained attributed mic for 2 s, 5-minute cooldown after dismissal. Unattributed activity needs a running meeting app as corroboration (`audioActivityDetector.js`).
- **Auto-end** (`meetingAutoEndController.js`):
  - Ownership mode: the external app releases the mic and the system channel stays quiet for 5 s. Needs at least 30 s of prior external mic hold.
  - Fallback mode: both channels silent for 60 s, or 10 s after the meeting app exits.
  - RMS thresholds: mic 0.006, system 0.004.
- **System-audio handover:** a fallback capture path takes over if the helper missed 1 s of audible audio (`meetingSystemAudioHandover.js`).

### Tests
- `test/helpers/*.test.js` covers the echo-leak detector, mic gate, holdback, auto-end, watchdog, timeline, handover, live speaker ID and speaker merge.
- A diarization eval script exists, but its fixtures are absent from the repo.

## What neither repo covers well (Deino must do better)
- Real-time-safe IOProc + SPSC ring (take this from Handy/Anarlog).
- Sample-rate change listeners.
- Fragmented crash-safe recording (take this from Anarlog/Cap).
- Audio performance benchmarks.
- **Test whether a global tap survives route changes.**
