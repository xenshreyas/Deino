# deino-recording

## Role

The live recorder (FND-05b, CAP-02,
[ADR 0026](../../docs/adr/0026-rust-owned-two-lane-live-recording.md)): Rust
owns a recording from start to stop. It creates the meeting, records the
microphone and system audio as separate crash-safe tracks (`deino-audio`),
transcribes them in two lanes, Me and Them, removes the call's echo from the
microphone lane, and saves the transcript into `transcript_segments` as it is
spoken. The UI only watches.

## Owns

- `RecordingService` (`service.rs`): one recording at a time; `start`
  (meeting row `recording`/`live`, its folder, the Me and Them speakers),
  `pause`, `resume`, `stop` (both tracks end together, lanes flush, timing
  set, `StopRecording` → `processing`, which starts the pipeline), `state`.
  The saved transcript is read through `deino_core::transcript` (TRN-01).
  `StartRequest` carries the title and the calendar event a recording is
  for. CAP-07/CAP-10 (ADR 0039): `resume_meeting` (`service/resume.rs`: a
  finished meeting back to `recording`, new audio after the recorded
  length), and in `service/controls.rs` the microphone switch, the
  system-audio toggle, the levels and the lanes' activity.
- `Session` (`session.rs`, `session/`): a recording's parts, started and
  stopped in order (router → engines → recorder → capture → meeting row →
  live pipeline).
- The live pipeline thread (`live/`): placed audio from the track writers →
  16 kHz → microphone/system pairs → echo canceller on speakers → per-lane
  VAD → lanes.
- Echo (`echo/`): `EchoMode` (`Cancel` on speakers, `Isolated` on
  headphones or with "I use headphones"), the `EchoCanceller` port and
  `SonoraAec` (WebRTC AEC3 through `sonora`, 10 ms frames, system track as
  reference).
- The bleed holdback (`bleed.rs`): OpenWhispr's `meetingMicHoldback`,
  ported; drops a Me segment only on a text match (`bleed::overlap`,
  re-exported from `deino_core::final_transcription::overlap`, which the
  after-meeting echo rule shares).
- Lanes (`engine/`, `lane.rs`): the `LaneEngines` / `LaneTranscriber`
  ports, `StreamingLane` (a `deino-stt` live session fed continuous audio)
  and `UtteranceLane` (the "Advanced" engines, cut at VAD boundaries).
- VAD (`vad.rs`): earshot per lane with hangover; segmentation and the
  holdback only, never gating what the engine hears.
- The transcript router (`transcript/`): drafts saved about once a second
  (`SegmentWriter::save_live`), holdback verdicts, batched final saves,
  `SegmentUpdate` events.
- The live vocabulary pass (`transcript/vocabulary.rs`, TRN-05):
  `CorrectedStore` and `CorrectedEvents` correct what the router saves and
  shows, and keep each original through `LiveVocabulary`.
- Record-only recordings (`Preferences::record_only`, CAP-08): no engines,
  no live pipeline; only capture and the track writers run.
- Transcribing finished tracks (`offline.rs`, `offline/utterances.rs`,
  CAP-08): `OfflineTranscriber` (the `final_transcription` stage's
  `TrackTranscriber`) over the `TrackDecoder` (`NativeDecoder`:
  `ExtAudioFile`) and `BatchEngines` ports; VAD utterances of at most 30 s,
  runs that stopped early read again in halves. A microphone track with an
  echo reference has the system track's echo cancelled from its decoded
  copy first (`offline/echo.rs`: `cancel_echo` over `SonoraAec`), unless
  `HeadphonesSetting` says headphones; the stored file is never changed.
- Imported files (`import.rs`, CAP-11): `NativeIngest`, `deino-core`'s
  `MediaIngest` over `deino-audio::media` (`audio.m4a`, and a video kept as
  `video.<ext>`). `examples/import_bench.rs` times a watched-folder import
  end to end with the real engine.
- Launch recovery (`recovery.rs`): meetings left in `recording`.
- Recording health (`monitor.rs`, `monitor/watch.rs`, CAP-10): the pure
  `HealthMonitor` (no audio 5 s per track, a speaking microphone with no
  transcript 30 s, the engine lost its stream, disk under 2 GiB or
  256 MiB) and the watcher thread that puts the open issues in the
  snapshot; `activity.rs` (what each lane hears and transcribes, also read
  by auto-stop); `state.rs` (the snapshot cell, announced under its lock).
- The events port (`events.rs`): `RecordingSnapshot` (with the open
  `issues`, the microphone, the call-audio switch and the calendar event),
  `SegmentUpdate`, `HealthNotice`, `AudioLevels`.
- The start observer port (`observer.rs`): `StartObserver`, told of each
  start once the meeting exists (the app links its calendar event, ORG-03),
  set with `RecordingService::observing_starts`; its title replaces the
  snapshot's.
- Capture backends (`backend.rs`): `DeviceCapture` and `MockCapture`
  (`DEINO_MOCK_AUDIO`).

## Does Not Own

- Devices, rings, track files, finalize: `deino-audio`.
- Engines and worker processes: `deino-stt`; the app's `stt/lanes.rs`
  implements `LaneEngines` over its engine pool and settings.
- The post-meeting work (finalize, diarize, summary): `deino-pipeline`.
- Tauri events, the tray, the record panel and notifications: the app's
  `features/recording` and `tray.rs`.
- When to stop on its own: `deino-detect::autostop` (CAP-07) asks the app,
  which calls `stop`.

## Invariants

- No dependency on `tauri` or `deino-net`.
- Nothing here runs on a real-time callback; the echo canceller and VAD run
  on the live pipeline's thread.
- The stored microphone track is never processed; only the lane's copy is.
- Engines always get continuous audio, so segment times are recording times
  on the shared timeline.
- Every saved segment carries its channel (`mic` for Me, `system` for Them)
  and its lane's speaker.
- Nothing is dropped as echo on audio correlation alone.
- Logs carry IDs, counts and states, never transcript text or titles.
- The holdback decides on transcript progress (the Them lane's watermark),
  not on wall time; a 20 s wall-clock cap is only a safety net.
- Only a track that is not delivering (capture status) is padded with
  silence; a late one is waited for.

## Tests

```bash
cargo test -p deino-recording                     # unit tests, service, fixture end to end
cargo test -p deino-recording --test live_fixture -- --ignored   # ten minutes on speakers (~70 s)
cargo test -p deino-recording --test record_only_cpu -- --ignored --nocapture   # CAP-08 CPU (macOS, ~1 min)
```

`tests/capture_modes.rs` records the fixture with live transcription off
(no engine may open; segments appear only once the `final_transcription`
rules run after stop) and with a live vocabulary (every saved and shown
line corrected, every original kept).

`tests/live_fixture.rs` plays `fixtures/audio/one-on-one/{mic,system}.m4a`
through mock capture, the real writers (fake encoder), the echo canceller,
both lanes (scripted fake engine) and the router into SQLite, and checks
channels, timing and the echo rate (speakers: the system track mixed into
the microphone 40 ms late at −9 dB). It is paced by sample count
(`tests/support/paced.rs`: host time advances only as far as the lanes have
taken audio, sources stop exactly at the clip's length), so a loaded machine
gives the same transcript as an idle one. `tests/controls.rs` covers Resume,
device switches and the disk-full issue. `tests/aec_allocations.rs` pins the
canceller's per-frame allocations; `tests/service.rs` covers the state
machine and launch recovery.
