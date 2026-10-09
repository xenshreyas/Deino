# Handy STT: patterns to port into Deino (CAP-01, CAP-02, FND-05)

All paths are relative to `~/Documents/GitHub/deino-references/Handy/src-tauri/` (MIT).

## Key facts
- **Version:** `transcribe-cpp` 0.3.1 (`Cargo.toml:81-83`, `features=["serde"]`), plus `metal` on macOS (`Cargo.toml:142`). ONNX Parakeet goes through `transcribe-rs` 0.3.8 instead (a separate path).
- **Worker process:** all native transcribe-cpp work runs in a **child worker process** (`src/engine_supervisor/{mod,protocol,worker,supervisor}.rs`), so a GGML abort, GPU fault or hang only kills the worker.
  - Wire format: `[u32 json_len][json][u32 pcm_len][f32 LE pcm]`.
  - The worker takes over stdout for the protocol and points fd 1 at stderr.
  - Deadlines scale with audio length. If the GPU worker crashes, Handy retries once on CPU. Cancel = kill.
- **Load:** `Model::load_with(path, &ModelOptions{backend, device})`, then `model.session()`. Device keys use the name on Metal. Set `GGML_METAL_NO_RESIDENCY=1` before Metal init (`lib.rs:626-638`). The first device listing compiles the Metal shaders (about 15 s), so Handy pre-warms it on a background thread (`lib.rs:1028-1042`).
- **Streaming API:**
  ```
  session.stream(&RunOptions,&StreamOptions)
  stream.feed(&pcm)   -> StreamUpdate{committed_changed,tentative_changed,revision,input_received_ms,audio_committed_ms,buffered_ms}
  stream.text()       -> {committed,tentative,full}
  stream.finalize()
  stream.reset()
  ```
  - Batch: `session.run(&pcm,&RunOptions)`.
- **Lifecycle** (`managers/transcription.rs:844-1101`):
  - A `StreamRouter` channel queues frames until the stream is ready.
  - Finalize goes through the same FIFO channel.
  - `Ok(None)` (stream lost) means batch transcription on the retained buffer instead (`actions.rs:651-660`).
  - committed = the append-only prefix; tentative = the part that may still be rewritten.
- **Timestamps:** Handy **never reads token timestamps**. It also **VAD-gates audio before streaming**, so stream time ≠ wall-clock time. For meetings, feed continuous audio and read token timestamps from the crate API (inspect the crate source).
- **Idle unload:** a watcher every 10 s, default unload after 5 minutes, never mid-recording (`transcription.rs:298-367`). Unloading kills the worker, which frees all memory.
- **Catalog:** Parakeet Unified entry at `src/catalog/catalog.json:8-40`.
  - Revision `7e948f21b7bdbac698d3318db9d350f1096f3b6c`. Default quant Q8_0 (731357568 B, sha256 `4b50b6dd862bf6e346929aaf4f5eaacec003bfa3f56462d6c874b41ef2f38795`).
  - Mirror URL: `{mirror}/{repo_id}/{revision}/{filename}`.
- **Downloader:** resumable Range downloads, size cap, stall timeout, sha256 in `spawn_blocking`, with a test suite (`managers/model/download.rs:56-380`, `model/download/tests.rs`). The HF path (`managers/model.rs:1922-2179`) **lacks sha256**, so Deino must add it.
- **Capture:**
  - The cpal callback writes into an `rtrb` SPSC ring (2 s), with pre-touched pages. It does no allocation, locking or logging, and counts overruns in an atomic (`audio_toolkit/audio/recorder.rs:426-547`).
  - The consumer loop drains every 10 ms; the stop handshake respects a pause boundary (`:914-1043`).
  - The first-sample readiness signal fires on the first real sample (`:376-391`).
  - Resampler: rubato `FftFixedIn` with a 1024-sample input chunk, exact reframing, delay-line flush and `reset()`, with tests (`audio/resampler.rs`).
  - VAD: Silero v4 with 30 ms/480-sample frames and threshold 0.3, or pure-Rust Earshot. `SmoothedVad` uses prefill 450 ms, onset 60 ms, hangover 450/1650 ms (`audio_toolkit/vad/*`).
  - Device fallback: clamshell > selected > default, cache, retry once, fall back to default (`managers/audio.rs:448-747`).
- **Performance:** `StreamPerf` logs RTF and backlog (`buffered_ms`, `input_received_ms - audio_committed_ms`) every 5 s (`transcription.rs:1541-1647`). Headless benchmark: `handy -f file.wav --model ID --repeat N --json` (`cli.rs`, `lib.rs:498-622`).
- **Tests:** recorder ring and pause boundary, resampler, VAD, protocol and downloader tests run without hardware. The supervisor end-to-end tests are `#[ignore]` with env vars, and faults can be injected with `HANDY_WORKER_FAULT`.
- **Do NOT port output muting.** Muting output would silence the system-audio capture.

## Gaps Deino must build itself
- reading token timestamps
- system-audio capture
- **two concurrent streams** (mic and system lanes)
- multi-hour backlog handling
