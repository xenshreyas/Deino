# 0026. Record live meetings in Rust, in two lanes, with WebRTC AEC3 against echo

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** FND-05b (Rust-owned live persistence), CAP-02 (Me/Them channel
  separation); `docs/references/audio-capture.md`,
  `docs/references/audio-muesli-openwhispr.md`, `docs/references/audio-anarlog.md`

## Context

Until now the record screen owned a recording. It started the legacy cpal +
process-tap capture in `audio/`, mixed microphone and system audio before one
transcription lane, kept the transcript in React state, and saved it on stop
through `api_save_transcript`, with an IndexedDB copy and a 30 s checkpoint
saver as crash insurance. A crash lost the meeting row and everything since
the last checkpoint; "Me" and "Them" could not be told apart; a reload
mid-recording lost the transcript on screen; and the summary was started by
the meeting page, so stopping from the tray never produced one.

FND-05a gave us crash-safe two-track writers (ADR 0014), CAP-01 the
streaming engine in worker processes (ADR 0016), FND-07 the segment and
lifecycle ports, and FND-08 a pipeline that starts when a meeting enters
`processing` (ADR 0018). What was left was to own the live path in Rust and
to keep the call's echo out of the microphone lane.

## Decision

- **Rust owns the recording** (`crates/deino-recording`, plugin `recording`:
  `start`, `stop`, `pause`, `resume`, `state`, `transcript`; events
  `RecordingState`, `LiveSegment`, `Health`). `start` creates the meeting
  (`recording`, `source = live`, its folder, a Me speaker linked to the
  user's person and a Them speaker), starts the writers and capture, and
  returns the ID; `stop` closes both tracks at one end, flushes the lanes,
  sets the timing and moves the meeting to `processing`, which enqueues
  `finalize_audio` (order 10, new), `diarize`, `legacy_summary` (now
  honouring `Settings::ai.auto_summary`), `auto_tag`, `auto_folder`. The UI
  is a viewer: TanStack Query over the saved segments plus `LiveSegment`
  events. The tray, ⌘⇧R, the palette, the sidebar and meeting detection all
  call the same service.
- **Two lanes.** Microphone → Me, system → Them, each with its own VAD
  (earshot, for segmentation and the holdback only; the engine always gets
  continuous audio, so token times are recording times) and its own
  `deino-stt` live session (one worker per lane). Every segment carries its
  `Channel` and times on the shared track timeline (sample index is time).
- **Persistence while speaking.** A transcript router thread saves the open
  segment of each lane about once a second (`SegmentWriter::save_live`, an
  upsert), and closed segments in one transaction per batch, so `kill -9`
  loses at most a few seconds of transcript and the meeting is recovered at
  launch (`recover_interrupted`: repair the tracks, set timing from the
  durable audio, move to `processing`; a meeting with no audio fails).
- **Capture** (`deino-audio::capture`): cpal microphone on its own thread
  with a start timeout and device ranking (built-in preferred over a
  Bluetooth headset in call mode), the Core Audio process tap through
  cidre (own PID excluded, stable aggregate UID
  `com.xenshreyas.deino.system-audio-tap`, `tap_auto_start = false`), RT-safe
  callbacks (preallocated SPSC rings, no allocation, lock or log, Mach host
  stamps), one resample per track (native → 48 kHz in a conditioner thread;
  48 → 16 kHz for the lanes). A supervisor thread handles default-device
  and device-list listeners (1 s debounce), `NSWorkspace` sleep and wake
  plus a wall-clock tick-gap check, and a stall watchdog with backoff
  (2 s, 5 s; 4 attempts; 15 s cooldown).
- **Echo: WebRTC AEC3 through `sonora` 0.2.0** (pure Rust port, BSD-3-Clause)
  on the live pipeline's thread, 10 ms frames, system track as reference.
  Behind it, OpenWhispr's text holdback (3 s hold, drop only on a text match
  above 0.6 against up to three merged Them segments, never on audio
  correlation). On headphones (route probe) or with "I use headphones" on,
  both are skipped and the microphone is marked isolated. The stored
  microphone track is never processed.

## Measurements

- **The global tap is upstream of output routing.** With `tap_probe`, a tone
  played by another process on a non-default HDMI output and on the default
  speakers was captured both ways, and our own process's tone was not (peak
  0). The aggregate device's IO cycle runs in silence (about 48 000 frames/s,
  ~94 callbacks/s), so the watchdog may treat frozen frames as a stall. We
  therefore do not rebuild the tap on an output route change; a route change
  only re-probes speakers vs headphones (two agreeing probes) for the echo
  mode. AirPods switching mid-call could not be tested here (manual QA).
- **Echo removal.** Synthetic echo (LCG noise, delayed and attenuated)
  cancels by more than 20 dB. On the one-on-one fixture with the system
  track mixed into the microphone 40 ms late at −9 dB, 0 of 8 stored Me
  lines are echo; over ten minutes (looped), 0 of 80, with 8 holdback
  discards. Acceptance was under 5 %.
- **Alignment.** Both tracks are placed on one host-time timeline; the
  fixture end-to-end test finds every Me and Them line within 800 ms of the
  truth with the 400 ms fake engine (the timeline itself is sample-exact;
  CAP-02's 50 ms bound is checked by the `deino-audio` timeline tests).
- **Allocations.** Our wrapper allocates nothing per frame; `sonora` 0.2.0
  allocates about 35 small vectors per 10 ms frame without growth (pinned at
  36 by `tests/aec_allocations.rs`). It runs off the real-time threads, so
  this is a deviation from "allocation-free steady state", not an RT hazard.
- **CPU.** `stt_bench` (release, Metal, real-time paced, worker processes
  sampled with `ps`) on the 54 s fixture: one lane on the mixed track 3.3,
  4.3, 7.7 CPU-s; two lanes (mic + system) 9.9, 16.6, 8.7 CPU-s; ratios
  2.95, 3.85, 1.13, at load averages of 14–38 from parallel builds on the
  same machine. Too noisy to confirm the ≤ 1.6× target; repeat on a quiet
  machine (follow-up).

## Alternatives

- **`webrtc-audio-processing` crate** (BSD-3 plus abseil Apache-2.0): the
  reference C++ implementation, but its bundled build needs meson, ninja,
  pkg-config, objcopy and bindgen, none of which our toolchain carries, and
  a C++ build per target. Rejected for build cost; reconsider if `sonora`
  lags upstream fixes.
- **DTLN-aec** (MIT upstream): a learned canceller that handles non-linear
  echo, but the ONNX conversion we found (anarlog's) has unclear provenance,
  it needs an ONNX runtime on the hot path and costs more CPU than AEC3.
- **Holdback only** (OpenWhispr's default): cheap, but it can only drop whole
  lines after the fact, so partial echo inside a real Me line survives.
- **Keep the frontend saver** with a Rust checkpoint: still loses the meeting
  row on a crash and still needs the window open to save.
- **Rebuild the tap on every route change** (muesli): unnecessary for a
  global tap (measured above) and it drops audio during the rebuild.

## Consequences

- Recording no longer depends on the window: the tray, detection and a
  reload all see the same recording; a crash leaves a recoverable meeting.
- The legacy recorder is gone (`audio/{recording_*, pipeline, stream,
  capture, incremental_saver, …}`, `useRecordingStart/Stop`, the IndexedDB
  recovery, the record-screen contexts), as is NOT-01's draft-and-adopt:
  the record screen's notepad writes straight into the live meeting.
- Two worker processes run during a recording (about 1.5 GB at Q8_0).
- Follow-ups: upstream a scratch-buffer patch to `sonora`; the CPU
  measurement on a quiet machine; live speaker labels from
  `deino_diarize::online::LiveSpeakerTracker`; the WebKit content process's
  own playback is not excluded from the tap; the "Save location" setting no
  longer applies to live recordings (they live under the app data folder).
- Revisit the echo choice if field reports show residual echo on speakers
  that the holdback misses, with DTLN-aec as the next candidate.
