# Audio, transcription and diarization references for Deino (PARTIAL)

Status: INCOMPLETE. This file contains only the Deino baseline audit. The run ended before the reference-repo research came back (Handy, anarlog, muesli, openwhispr, sherpa-onnx, Cap and zed). The per-topic "Best implementation / Runner-up / Adopt / Copy vs reimplement" sections and the "Recommendations by card" section still need to be written.

## Deino baseline audit (frontend/src-tauri/src/audio/)

### System audio: `capture/core_audio.rs`
- Uses a cidre process tap (`TapDesc::with_mono_global_tap_excluding_processes`, about lines 95-100) and a private, tap-only aggregate device (lines 128-150). The device is named `meetily-audio-tap` (line 139) and gets a random UUID, so a stale-device sweep cannot find it by prefix. Rename it to `deino-audio-tap-<uuid>` (section 3C).
- Real-time violations in the IO proc:
  - It calls `device.nominal_sample_rate()` (a HAL property read) on every callback (lines ~165-172).
  - `process_audio_data` takes a `std::sync::Mutex` (`waker_state.lock()`) to wake the consumer (lines ~290-302).
- After more than 10 consecutive ring overflows, the stream ends itself (`should_terminate`) and does not recover.
- The `Stream` impl yields one `f32` per `poll_next`, which costs one waker round-trip per sample. Batch the reads instead.
- Nothing listens for default-output changes, tap invalidation or sleep/wake. A sample-rate change only updates an atomic, and nothing re-creates the format or resampler.

### Microphone: `stream.rs:237-300` and `pipeline.rs:399-660` (`AudioCapture::process_audio_data`)
- The cpal callback does all of the following, which breaks the section 3C rules:
  - allocates `Vec`s for i16/i32/i8 conversion;
  - locks three or four `std::sync::Mutex`es (resampler, noise suppressor, HPF, normalizer);
  - runs rubato `SincFixedIn` resampling (512-sample chunks) and noise suppression;
  - logs;
  - sends to an unbounded tokio mpsc.
- Target fix: the callback only copies frames into an `rtrb` or `ringbuf` SPSC ring. All DSP moves to a worker thread.

### Mixing and VAD: `pipeline.rs`
- `AudioMixerRingBuffer` (lines 31-158) mixes mic and system in 600 ms windows. It zero-pads whichever side is short and drops the oldest samples past 8 windows. This makes the two tracks drift and lose alignment.
- VAD (`vad.rs`, silero_rs at git rev 26a6460, thresholds 0.50/0.35, redemption 500 ms) runs on the mixed signal (`pipeline.rs:859-890`), and every segment is tagged `DeviceType::Microphone`. CAP-02 needs two lanes, so VAD has to run before the mix, once per channel.
- The channel into the pipeline is an unbounded mpsc (`pipeline.rs:1001`), so there is no backpressure.

### Transcription
- The `TranscriptionProvider` trait (`transcription/provider.rs:49-73`) is batch-only: `transcribe(Vec<f32>) -> TranscriptResult {text, confidence, is_partial}`. It has no session or streaming API and returns no token timestamps.
- CAP-01 needs a streaming session trait: `start_session`, `push(&[f32])`, `partial`, `finalize` returning words with `start_ms`/`end_ms`.

### Persistence: `incremental_saver.rs`
- Checkpoints are 30 s mp4 chunks (line 21), joined with an ffmpeg concat at the end (lines 290-337). A crash can therefore lose up to 30 s. FND-05 allows at most about 5 s, so the interval must drop to 5 s or less, or the recording should switch to fragmented MP4 or raw PCM appends.
- Only the mixed stream is saved. There are no per-track files.

### Detection: `system_detector.rs`
- Listens to `DEVICE_IS_RUNNING_SOMEWHERE` on the default device (lines 89-93), debounced at 500 ms (lines 96-128).
- Lists apps through `ca::System::processes()` and `is_running_output` (lines 358-380). That is output, not input. CAP-06 needs `is_running_input` plus the bundle ID, and camera detection via CMIO.

### Devices: `device_monitor.rs`
- Has a HW_DEVICES listener plus a 2-5 s poll (lines 142-160, 272-396). There is no `kAudioHardwarePropertyDefaultInputDevice` or default-output listener.

## Still to do
The per-topic reference analysis (items 1-9 of the brief) and the per-card recommendations for FND-05, CAP-01, CAP-02, CAP-03, CAP-04, CAP-06, CAP-07, CAP-08 and CAP-10.
