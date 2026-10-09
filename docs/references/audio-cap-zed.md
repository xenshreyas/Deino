# Cap and Zed: audio patterns to re-implement. REFERENCE ONLY, never copy code.

**Licence:** Cap and Zed are copyleft. Agents may read them to learn the patterns, then write Deino's own implementation from scratch.

Paths: Cap = `~/Documents/GitHub/deino-references/Cap/`, Zed = `~/Documents/GitHub/deino-references/zed/`.

**Note:** Cap uses ScreenCaptureKit audio (via `cidre`) for system audio, **not** a process tap. Its value for Deino is in timing, gaps, health and crash safety, not the capture API.

## 1. Sample-count timeline plus a gap/overlap tracker
Cap `crates/recording/src/output_pipeline/core.rs:739-1070`.
- Derive time from cumulative sample counts, converting on the running total so there is no drift.
- Compare that with capture-timestamp elapsed time, capped at wall clock + 100 ms.
- Fill gaps longer than 70 ms (wired) or 160 ms (wireless) with silence, in chunks of at most 1 s.
- Trim overlaps longer than 5 ms.
- Pad the tail up to the stop point.

## 2. Epoch anchoring for the system track
Cap `core.rs:1683-1697`, `studio_recording.rs:3444-3460`.
- Intermittent sources (system audio) get silence from the recording epoch, so mic and system files both align at t=0.
- Gaps on an epoch-anchored track are expected, so they are not reported as health problems.

## 3. Crash-safe fragmented M4A per track
Cap `crates/enc-ffmpeg/src/mux/fragmented_audio.rs:39-80`.
- MP4 flags: `frag_keyframe+empty_moov+default_base_moof+skip_trailer`, 2 s fragments, flush packets.
- Audio-only streams need a fragment *duration*, because they have no keyframes to fragment on.

Segmented variant: `mux/segmented_audio.rs:11-33, 193-315`.
- Rotate `fragment_NNN.m4a`; on each rotation, write the trailer and fsync.
- Write `manifest.json` atomically: temp file, fsync, rename, fsync the directory.

## 4. Startup recovery
Cap `crates/recording/src/recovery.rs:285-445`, `mux/fragmented_mp4.rs:7-28`.
- The meta `status` is the dirty flag; take a per-recording lock.
- Check the fragment tail by walking the top-level boxes: the file is valid only if it ends with `moof` + `mdat`.
- Remux the complete fragments.

## 5. Typed health-event bus, stall budget, disk monitor
Cap `core.rs:390-697`, `crates/utils/src/disk_space.rs:5-8, 150-190`.
- **Events:** FrameDrop, AudioGap, SourceRestarting/Restarted, Stalled, DiskSpaceLow/Exhausted, DeviceLost, SourceAudioReset.
- `emit_health` logs the event and `try_send`s it to a bounded channel of 32, so it never blocks.
- **Stall budget:** retry for up to 50 ms, then drop the frame, count it and emit a health event.
- **Disk monitor:** poll statvfs every 2 s and emit only on state change.
  - Warn at 200 MB free, stop at 50 MB.
  - Before recording, warn below 2 GB.

## 6. Clock and latency
- **Mach host time:** Cap `crates/timestamp/src/macos.rs:1-74`. Get the timebase direction right: ns × denom / numer. The bug in the other direction adds about 1736x the time on Apple Silicon.
- **Input latency compensation:** Cap `crates/audio/src/latency.rs:574-641`, `feeds/microphone.rs:1309-1324, 1661-1667`. Subtract device latency + safety offset + stream latency, capped at 0.5 s.
- **Transport type:** detect Bluetooth / USB / built-in via the transport type (`latency.rs:749-763`).
- **OBS-style per-source smoothing:** `crates/timestamp/src/master_clock.rs:260-339`. Its tests at lines 361-900 are a good template.

## 7. Real sample-rate detection and resampler rebuild
Cap `feeds/microphone.rs:82-345`, `sources/microphone.rs:113-209, 448-495`.
- Estimate the real rate from callback frame counts. Snap to a standard rate, and require 3 agreeing windows before switching.
- Rebuild the resampler when the format changes.
- Stamp the output with the input's capture time, **not** capture time minus resampler delay. Subtracting the delay caused overlaps.

## 8. Device changes
- **Zed:** `crates/livekit_client/src/livekit_client/playback.rs:825-1045` registers Core Audio property listeners for the default input/output device and the stream format, and turns them into an async stream with a reopen loop (lines 238-411).
  - Error classification: `crates/audio/src/audio_pipeline.rs:261-285` ignores Xrun and DeviceChanged and acts only on the first fatal error.
- **Cap stall watchdog:** `crates/recording/src/sources/microphone.rs:23-41, 380-660`.
  - Channel timeout of 100 ms (wired) or 200 ms (wireless).
  - After a 2 s stall, reconnect with exponential backoff up to 30 s.
  - Insert keepalive silence only after 1 s.
- **Mute** sets samples to zero but keeps the timeline running.

## 9. Echo cancellation (if needed for laptop speakers)
Zed `playback.rs:50-80`: a WebRTC APM with AEC, AGC2, high-pass and noise suppression, processed in 10 ms chunks.
- The playback stream is fed as the reverse (reference) stream.
- **For Deino:** feed the **system track as the reference** and the mic track as the capture, to remove system-audio bleed from the mic. This is a better approach than text-dedup for CAP-02. Check the licence of the `webrtc-audio-processing` crate (BSD-like) before using it.

## 10. Testing
Cap `crates/recording/src/test_sources/` and `tests/{sync_matrix,recovery,segmented_pipeline}.rs`.
- Synthetic sources: sine, chirp, noise, timestamp beeps.
- A hardware-free sync matrix across sample rates, channel counts and jitter/gap problems.
- Corrupt-fragment recovery fixtures.
- Debug-only fault-injection points (`crates/fail`).

## Cautions
- Cap's mic callback allocates per buffer. Deino must use a preallocated SPSC ring in the IOProc (as Handy does).
- AVAssetWriter is not crash-safe without a movie fragment interval. Use the ffmpeg fragmented-mp4 approach instead.
