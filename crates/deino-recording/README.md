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
  set, `StopRecording` → `processing`, which starts the pipeline), `state`,
  `transcript`.
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
- The bleed holdback (`bleed.rs`, `bleed/overlap.rs`): OpenWhispr's
  `meetingMicHoldback`, ported; drops a Me segment only on a text match.
- Lanes (`engine/`, `lane.rs`): the `LaneEngines` / `LaneTranscriber`
  ports, `StreamingLane` (a `deino-stt` live session fed continuous audio)
  and `UtteranceLane` (the "Advanced" engines, cut at VAD boundaries).
- VAD (`vad.rs`): earshot per lane with hangover; segmentation and the
  holdback only, never gating what the engine hears.
- The transcript router (`transcript/`): drafts saved about once a second
  (`SegmentWriter::save_live`), holdback verdicts, batched final saves,
  `SegmentUpdate` events.
- Launch recovery (`recovery.rs`): meetings left in `recording`.
- The events port (`events.rs`): `RecordingSnapshot`, `SegmentUpdate`,
  `HealthNotice`.
- Capture backends (`backend.rs`): `DeviceCapture` and `MockCapture`
  (`DEINO_MOCK_AUDIO`).

## Does Not Own

- Devices, rings, track files, finalize: `deino-audio`.
- Engines and worker processes: `deino-stt`; the app's `stt/lanes.rs`
  implements `LaneEngines` over its engine pool and settings.
- The post-meeting work (finalize, diarize, summary): `deino-pipeline`.
- Tauri events, the tray and notifications: the app's `features/recording`.

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

## Tests

```bash
cargo test -p deino-recording                     # unit tests, service, fixture end to end
cargo test -p deino-recording --test live_fixture -- --ignored   # ten minutes on speakers (~2.5 min)
```

`tests/live_fixture.rs` plays `fixtures/audio/one-on-one/{mic,system}.m4a`
through mock capture, the real writers (fake encoder), the echo canceller,
both lanes (scripted fake engine) and the router into SQLite, and checks
channels, timing and the echo rate (speakers: the system track mixed into
the microphone 40 ms late at −9 dB). `tests/aec_allocations.rs` pins the
canceller's per-frame allocations; `tests/service.rs` covers the state
machine and launch recovery.
