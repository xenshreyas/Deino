# 0016. Run speech-to-text in supervised transcribe-cpp worker processes

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** CAP-01

## Context

CAP-01 makes Parakeet Unified EN 0.6B (GGUF, through `transcribe-cpp` 0.3.1)
the default engine. Three facts shaped the integration
(`docs/references/spike-stt-streaming.md`, `docs/references/audio-stt.md`):

- ggml aborts the whole process on a failed `GGML_ASSERT`, and Metal driver
  faults and hangs happen. A meeting recorder must not lose the recording to
  its transcriber.
- A loaded model serves one stream at a time (`Error::Busy`), and CAP-02
  needs a microphone and a system lane streaming concurrently.
- `whisper-rs` and `transcribe-cpp` each link their own unprefixed ggml; the
  linker keeps one copy and transcribe-cpp aborts at runtime. Re-launching the
  same binary as a worker does not avoid it, because the worker links what the
  app links.

The buffered Parakeet stream's default window (5600/1040/1040 ms) gives about
2 s to first text; the spike measured 5600/560/320 at 0.85 s onset-to-text
p50 with the same WER. Batch `run()` can stop emitting text partway through
long files, and the stream's internal PCM buffer is never trimmed.

## Decision

- A new crate, **`deino-stt`**, owns everything native: the worker process,
  its framed protocol, the supervisor, live sessions, the engine pool and the
  metrics. It has no network dependency (checked in `scripts/clippy.sh`);
  models arrive as verified paths from `deino-core::models`.
- **The app executable is also the worker** (`deino --transcribe-worker`,
  Handy's design). `EngineSupervisor` owns one worker: an owner thread with a
  command queue, deadlines that scale with the audio, a retry on the CPU after
  a GPU crash or hang (in a worker that never loads a GPU driver), and cancel
  by killing that worker's own PID. A lost stream reports `StreamLost`; the
  session re-transcribes the audio it retained in one batch run and opens a
  new stream where the old one stopped.
- **One worker per lane** (`EnginePool`), plus one for batch work (import,
  retranscription), so streams never block each other. Engines load lazily,
  stay loaded while a recording pins them, and unload after 5 idle minutes.
- **Live audio is continuous**, never VAD-gated, so token times are meeting
  times (`stream start + t0`). Sessions assemble committed tokens into
  segments (pause, decoded silence, length limits), roll the stream every
  3–5 minutes at a silence, and log `StreamPerf` (RTF, backlog, queue lag,
  first-text and token latency) with numbers only.
- **The live window is 5600/560/320 ms.** Batch work runs on VAD segments of
  25 s or less.
- **`whisper-rs` is removed.** Whisper models (the legacy whisper.cpp `.bin`
  files, which transcribe-cpp loads) run in the same supervised workers.

## Alternatives

- **In-process transcribe-cpp** (two `Model`s in the app): about 0.45 GB less
  memory for two lanes, but one crash domain with the recording. Rejected;
  a single worker holding two models remains a memory fallback for 8 GB Macs.
- **One long stream per meeting** (no rotation): works and does not drift over
  82 minutes, but its PCM buffer (about 230 MB per hour) and per-chunk text
  rebuild grow linearly.
- **Utterance-scoped, VAD-gated streams**: bounded, but VAD decides the
  timeline and cuts words; continuous audio with rotation keeps exact times.
- **A separate sidecar binary** for transcribe-cpp, keeping whisper-rs in the
  app: avoids the link clash, but adds a second binary to sign and ship and
  keeps two ggml copies. Rejected in favour of dropping whisper-rs.

## Consequences

- A native crash or hang costs at most the audio since the last closed
  segment of one lane, and that audio is re-transcribed.
- Two Q8_0 lanes cost about 2 GB (two processes); Macs under 32 GB default
  to Q5_K_M.
- The first GPU open after an install or OS update compiles the Metal library
  (about 17 s); the app pre-warms it at startup and the hello timeout is 60 s.
- Supervisor faults are tested against real processes with a fake worker
  binary (`deino-stt-fake-worker`), so CI needs no model.
- Revisit when transcribe-cpp offers a trimmed buffered stream, or if memory
  on small Macs forces a shared-model worker.
