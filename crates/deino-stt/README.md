# deino-stt

## Role

Speech-to-text: Parakeet Unified EN 0.6B (and Whisper) through
`transcribe-cpp`, isolated in worker processes, with live stream sessions and
their metrics. CAP-01; ADR 0016.

## Owns

- `worker`: the worker process (`--transcribe-worker`): the request loop
  (`serve`), the protocol pipe on the real stdout, `_exit` on stdin EOF, the
  stderr logger and the raised thread QoS.
- `engine`: the `Engine` the worker drives. `TranscribeCppEngine` (the real
  one: load, warm-up, batch runs, streams that report committed-token deltas)
  and `scripted::ScriptedEngine` (test-support: loud 400 ms blocks become
  numbered words; `FaultPlan` crashes or hangs on cue).
- `protocol`: the typed request/response messages and their framing.
- `supervisor`: `EngineSupervisor`, the parent side of one worker: owner
  thread, deadlines scaled to the audio, GPU→CPU fallback, cancel by killing
  the worker (by its own PID only), lost streams reported as `StreamLost`.
- `transcriber`: the `BatchTranscriber`/`StreamingTranscriber` ports a
  session depends on; `FakeTranscriber` (test-support) for session tests.
- `session`: `LiveSession`, one live lane: FIFO feeds, continuous (never
  VAD-gated) audio, segment assembly from committed tokens, retained audio
  and batch fallback, stream rotation every 3–5 min.
- `pool`: `EnginePool`, one supervisor per lane plus one for batch work, lazy
  loading, pinning and idle unload.
- `perf`: `StreamPerf` (RTF, backlog, queue lag, first-text and token
  latency), logged without text. `bench`: the headless benchmark.
- `words`, `window`, `pcm`, `clock`: tokens to timed words, the streaming
  window (`StreamWindow::LIVE` = 5600/560/320 ms), 16 kHz time, and the
  monotonic clock port.
- `src/bin/fake_worker.rs` (test-support) and `examples/stt_bench.rs`.

## Does Not Own

- Downloading models: `deino-core::models` (this crate gets a path).
- Audio capture, mixing and resampling: the app's audio pipeline.
- Persisting segments and words: FND-05b (`transcript_segments`).
- Tauri events and commands: the app's `stt` module and `transcription` plugin.

## Invariants

- **No network.** No dependency on `deino-net`, `reqwest` or any HTTP client
  (§4.0); `scripts/clippy.sh` checks it with `cargo tree`.
- No dependency on `tauri`. No transcript text in any log line.
- Native transcription code runs only in a worker process. One worker holds
  at most one model and one stream; a second lane gets its own worker.
- Every worker is stopped through its own handle (stdin close, then kill by
  PID); nothing kills processes by name.
- Audio fed to a stream is continuous, so token times are on the recording's
  timeline (`stream start + t0`).
- The only `unsafe` is in `worker/stdio.rs` (fd duplication, `_exit`, the
  thread QoS class), each block with its safety argument.
