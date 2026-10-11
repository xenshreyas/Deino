# deino-audio

## Role

Crash-safe recording of the two audio tracks (microphone and system) into
fragmented AAC files that survive `kill -9` (≤ 1 s lost) and power loss
(≤ 5 s lost), launch recovery of unclosed recordings, and finalize into
`mic.m4a`, `system.m4a` and the mixed `audio.mp4`. No FFmpeg
([ADR 0014](../../docs/adr/0014-native-crash-safe-recording-format.md)).

## Owns

- The timeline rule **the sample index is time**: `TrackTimeline` places
  host-stamped blocks, fills gaps with silence and anchors a late track to
  the recording epoch.
- The real-time boundary: `RtSink` / `RtSource` over a preallocated `rtrb`
  ring per track.
- The ports `AudioSource`, `TrackEncoder` / `EncoderFactory`, `TrackSink`,
  `HostClock`, `DiskSpace`, and their adapters: `MockAudioSource`
  (`DEINO_MOCK_AUDIO` in debug builds), `CmafEncoderFactory` (macOS
  `AVAssetWriter` in CMAF segment mode), `PartFileSink`, `SystemHostClock`
  (mach time), `SystemDiskSpace`; fakes behind `test-support`.
- The writer thread (ring → timeline → 64 MiB backlog → encoder), the
  persister (F_FULLFSYNC and the manifest every 5 s) and `Recorder`.
- `recording.json` (`RecordingManifest`, versioned, atomically written).
- `fmp4::truncate_to_playable`, `recovery::recover_recording`,
  `finalize::finalize_recording`, and `decode::read_mono` (a finished
  track as mono PCM at any rate, priming trimmed; diarization reads 16 kHz).
- `HealthEvent` (`Stalled`, `Resumed`, `FramesDropped`, `DiskLow`,
  `StorageFailed`, and from capture `Recovering`, `Recovered`, `DeviceLost`,
  `BluetoothCallMode`, `SystemAudioUnavailable`, `Asleep`, `Awake`) on a
  bounded channel.
- Capture (`capture/`, CAP-02, [ADR 0026](../../docs/adr/0026-rust-owned-two-lane-live-recording.md)):
  the cpal microphone (`mic`: its own thread, a start timeout, device
  ranking that prefers the built-in microphone over a Bluetooth headset in
  call mode), the Core Audio process tap (`macos/tap`: own PID excluded,
  aggregate UID `com.xenshreyas.deino.system-audio-tap` plus one fallback,
  `tap_auto_start = false`), Core Audio and `NSWorkspace` listeners
  (`macos/listener`, `macos/power`), the output route probe, the
  conditioner per track (native rate → 48 kHz with rubato, pause ledger),
  and `Capture`, whose supervisor thread restarts sources on device
  changes (1 s debounce), stalls (watchdog with backoff), sleep and wake.
- The writer's tee of placed audio (`PlacedAudio`, position-stamped) that
  the live recorder transcribes.

## Does Not Own

- Meeting rows, `transcript_segments`, transcription and echo removal:
  `deino-recording` drives `Recorder` and `Capture`; the pipeline's
  `finalize_audio` stage calls `recover_recording` and `finalize_recording`.
- Where recordings live: the caller passes the directory (resolved from
  Tauri's `app_data_dir`).

## Invariants

- No dependency on `tauri` or `deino-net`.
- Real-time code (device callbacks) touches only the preallocated rings
  (`push`, `push_frames`): no allocation, lock, log or system call. Nothing on the real-time side wakes the writer.
- Only the writer thread touches an encoder; encoders are opened on it.
- The manifest is written after the audio it describes is synced, so it
  never claims audio that is not on disk.
- Every output file is written under a temporary name, synced and renamed.
- Deino's AAC files are decoded with ExtAudioFile or AVFoundation, which
  trim the 2112-frame encoder priming; symphonia 0.5 does not.
- Native encoding and finalize are macOS-only; everything else builds and is
  tested on every platform.
- Logs carry track names, counts and OS status codes, never audio content.

## Tests

`cargo test -p deino-audio` runs the unit tests (fakes, no hardware) and, on
macOS, `tests/finalize_round_trip.rs` (real encoder, alignment within 50 ms,
fixture audio). The real-time acceptance harnesses are ignored by default:

```bash
cargo test -p deino-audio --test kill_harness -- --ignored --nocapture --test-threads=1
cargo run -p deino-audio --example tap_probe -- 30   # the tap on real hardware (macOS)
```
