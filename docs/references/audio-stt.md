# Streaming STT, model management and speaker diarization: reference study for Deino

Status: complete. This file covers the transcription and diarization half of the audio work. The capture-side baseline audit of Deino is in `references/audio.md` in this same folder. Read that first for the current state of `frontend/src-tauri/src/audio/`.

Sources studied (all paths are under `~/Documents/GitHub/deino-references/` unless they start with `~`):
- `Handy/src-tauri/src/...` (MIT). The owner uses it daily. It runs Parakeet Unified EN 0.6B GGUF through `transcribe-cpp`.
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/transcribe-cpp-0.3.1/` and `transcribe-cpp-sys-0.3.1/` (MIT). These were fetched with `cargo fetch` in `Handy/src-tauri`. Below, `TC` means `transcribe-cpp-0.3.1` and `TCS` means `transcribe-cpp-sys-0.3.1`.
- `sherpa-onnx/` (Apache-2.0), including its official Rust crates in `sherpa-onnx/sherpa-onnx/rust/`.
- `anarlog/crates/...` and `anarlog/plugins/...` (MIT outside `enterprise/`; nothing below comes from `enterprise/`).
- `openwhispr/src/helpers/...` (MIT).

---

## 0. Findings that change the plan (read these first)

1. **The crate version in CAP-01 is out of date.** The plan says `transcribe-cpp = "0.1.3"`. Handy pins `transcribe-cpp = "0.3.1"` (`Handy/src-tauri/Cargo.toml:84-86, 159`), with the `metal` feature on macOS. The 0.3 API (`StreamOptions`, family extensions, `StreamUpdate`, `Capabilities`) is the one documented below. Pin `=0.3.1`.
2. **Handy never runs transcribe.cpp inside the app process.** All native transcribe.cpp code runs in a child process: the same executable re-launched with `--transcribe-worker`. That process is supervised for crashes, hangs and GPU faults, and falls back to CPU (`Handy/src-tauri/src/engine_supervisor/`). A `GGML_ASSERT` abort or a Metal driver fault therefore kills only the worker, never the recording. Deino should copy this design. It matters even more for a meeting recorder than for a dictation app.
3. **Only one stream can run per model at a time.** The C library allows at most one in-flight run or stream across all sessions of a `Model`. A second `stream()` or `run()` returns `Error::Busy` (`TC/src/lib.rs:22-33`, `TC/src/session.rs:293-325`). CAP-02 needs two live lanes (mic and system), so it needs **two models, which in practice means two worker processes**, or a time-multiplexing scheduler. See §1.
4. **Handy's live path gives text but no timestamps.** It reads only `Stream::text()` (`Handy/.../engine_supervisor/worker.rs:192-249`). Token and word timestamps come from `Stream::snapshot()`, which returns a full `Transcript` with `words` and `tokens` (`TC/src/session.rs:657`, `TC/src/result.rs`). Deino must call `snapshot()` after `finalize()` and send it over the worker protocol. Handy does not do this today.
5. **Parakeet takes no hotwords or vocabulary in transcribe.cpp.** `Feature::Vocabulary` is set only for whisper, qwen3_asr, granite, funasr_nano and moss (`TCS/src/arch/*/capabilities.cpp`, `model.cpp`). Custom words for Parakeet have to be applied as a fuzzy correction after decoding, as Handy does with `apply_custom_words` (`Handy/src-tauri/src/audio_toolkit/text.rs:151`).
6. **Handy does not hash-check Hugging Face downloads.** The sha256 check applies only to mirror and URL downloads (`Handy/.../managers/model/download.rs:56-121`). The HF path goes through a git fork of `hf-hub` and never calls `verify_sha256` (`managers/model.rs:1922-2181`). Deino should download from the HF `resolve/<revision>/<file>` URL with Handy's resumable HTTP downloader and **always** verify the sha256.
7. **Two libraries may fight over the same symbols at link time.** This is a risk to confirm with a test link early in CAP-01.
   - `whisper-rs` (Deino today, `frontend/src-tauri/Cargo.toml:179`) and `transcribe-cpp-sys` both link their own unprefixed static ggml (`TCS/bindings/rust/sys/build.rs`). Handy avoids the clash by running Whisper through transcribe-cpp GGUFs; its catalog lists `handy-computer/whisper-*-gguf`. Deino should do the same and drop `whisper-rs`.
   - Likewise, `sherpa-onnx`/`sherpa-rs` in static mode bundle their own onnxruntime, while Deino already links `ort = 2.0.0-rc.10`.
8. **There is a better diarization option than `sherpa-rs`.** anarlog's `crates/pyannote-local` + `crates/embedding` + `crates/voiceprint` (MIT) is a pure-Rust pyannote-3.1-style pipeline built on `ort =2.0.0-rc.10`, the exact `ort` version Deino already uses.
   - It returns **per-speaker centroids**, which CAP-04 needs. sherpa-onnx's C API does not expose them.
   - It also accepts known voiceprints to merge fragmented clusters, supports cancellation and progress, reads audio through a file-backed `AudioSource`, and ships a DER metric and an eval loop.
   - Recommendation: port it, record the choice in an ADR, and keep sherpa-onnx as the runner-up. If sherpa is used anyway, take the **official** `sherpa-onnx` crate 1.13.8 (Apache-2.0, k2-fsa), not the third-party `sherpa-rs` 0.6.8 (MIT, thewh1teagle). Both download prebuilt native libraries in `build.rs`.
9. **The default streaming window misses the CAP-01 latency target.** Parakeet Unified's default (L, C, R) context is 5600/1040/1040 ms (`TCS/include/transcribe/parakeet.h:70-97`), so the first text lands about C+R ≈ 2.1 s after speech. The "≈1.5 s" acceptance criterion needs a smaller (C, R) pair from the model's training menu: read GGUF keys `stt.parakeet.encoder.att_chunk_{left,chunk,right}_choices` and pass them via `StreamExtension::ParakeetBuffered`. Measure WER and latency on each candidate.
10. **Never drop a mic segment on audio evidence alone.** openwhispr's field logs show genuine local speech correlating 0.73–0.81 with the system stream during double-talk (`openwhispr/src/helpers/meetingMicHoldback.js:1-12`). CAP-02's "drop when the mic energy is mostly correlated with the system stream" rule would delete real speech. Audio evidence should only *hold a segment back*. A text match must confirm the duplicate before anything is dropped.
11. **transcribe.cpp can also diarize.** It ships NVIDIA Sortformer (4-speaker streaming diarizer, `TCS/include/transcribe/sortformer.h`) and a multitalker Parakeet bundle with an embedded Sortformer (`TCS/src/arch/parakeet/multitalker.cpp`). Neither produces speaker embeddings, so neither can drive CAP-04 voiceprints, and both cap out at 4 speakers. Treat them as a possible later CAP-03 live path, not the main one.

---

## 1. Streaming STT design

### Best found
**Handy and transcribe-cpp 0.3.1 together.** This is the engine the plan mandates, and the integration is production-hardened.

- **transcribe-cpp API surface** (`TC/src/`):
  - `Model::load_with(path, &ModelOptions { backend: Backend::{Auto,Cpu,CpuAccel,Metal,..}, device: Option<Device> })`.
    - `model.capabilities()` returns `Capabilities { native_sample_rate, languages, max_timestamp_kind, supports_streaming, supports_translate, supports_language_detect, max_audio_ms, .. }`.
    - Also on `Model`: `supports(Feature)`, `arch()`, `variant()`, and `backend()` (the backend actually bound; this is how a CPU fallback is detected) (`model.rs`).
    - `Model` is `Send + Sync`, `Arc`-backed and cheap to clone.
  - `model.session_with(&SessionOptions { n_threads, kv_type, n_ctx })` returns a `Session`. It is `Send`, not `Sync`, and every call takes `&mut self` (`session.rs:1-120`, `model.rs:298-315`).
  - Batch: `session.run(&pcm16k_f32, &RunOptions) -> Result<Transcript>` and `run_batch(&[&[f32]], ..)` (`session.rs:171-270`).
    - `RunOptions { task, timestamps: TimestampKind::{None,Auto,Segment,Word,Token}, pnc, itn, diarize, language, vocabulary, prompt, family: Option<RunExtension>, .. }` (`session.rs:35-80`).
  - Streaming: `session.stream(&RunOptions, &StreamOptions) -> Stream<'_>`, then `stream.feed(&pcm) -> StreamUpdate`, `stream.text() -> StreamText { full, committed, tentative }`, `stream.snapshot() -> Transcript` (words and tokens), `stream.finalize() -> StreamUpdate`, and `stream.reset()`. Dropping the stream abandons it (`session.rs:293-325, 545-689`).
    - `StreamUpdate { revision, input_received_ms, audio_committed_ms, buffered_ms, committed_changed, tentative_changed, is_final }` (`streaming.rs`).
  - `StreamOptions { commit_policy: CommitPolicy::{Auto,OnFinalize,StablePrefix}, stable_prefix_agreement_n, family: Option<StreamExtension> }`. For Parakeet Unified the extension is `StreamExtension::ParakeetBuffered(ParakeetBufferedStreamOptions { left_ms, chunk_ms, right_ms })`. Values must be multiples of 80 ms and appear in the model's training menu (`family.rs`, `TCS/src/arch/parakeet/model.cpp:2630-2720`).
  - Results: `Transcript { text, language, timestamp_kind, segments, words: Vec<Word{t0_ms,t1_ms,text,..}>, tokens: Vec<Token{p,t0_ms,t1_ms,text,..}>, speaker_segments, timings: Timings{load_ms,mel_ms,encode_ms,decode_ms} }`. All times are in ms relative to the start of the run or stream (`result.rs`).
  - Cancellation: `session.set_cancel_token(&CancelToken)`, honoured when the model supports `Feature::Cancellation` (`session.rs:138`).
  - Errors: `Error::{InvalidArgument, ModelLoad, OutOfMemory, Backend, Busy, Aborted{partial}, OutputTruncated{partial}, OutputRepetition{partial}, InputTooLong, ..}`. The last three keep a partial transcript (`error.rs:22-95`).
  - Threading rule: one in-flight compute per `Model` (finding 3). Backend init: `transcribe_cpp::init_backends_default()` or `init_backends_with(None, BackendMask::CPU)`, plus `transcribe_cpp::devices()`.
  - Hotwords: `RunOptions::vocabulary` exists but Parakeet ignores it (finding 5).
- **Handy's streaming wiring** (`Handy/src-tauri/src/managers/transcription.rs`):
  - `StreamRouter` (125-183): an `AtomicBool` fast path plus an mpsc of `StreamCmd::{Feed(Vec<f32>), Finalize(reply), Cancel}`. Feeds and finalize share one FIFO channel, so every frame is processed before finalize.
  - `start_stream` (844-863): token-guarded so two workers can never race (`StreamWorkerGuard`, 223-242).
  - `run_stream_worker` (865-1051): waits for any model load in progress, checks `supports_streaming`, builds `RunOptions`, then runs `engine.start_stream(run, StreamOptions::default(), on_progress)`. The `on_progress` callback emits `StreamTextEvent { committed, tentative }` (61-69, 1529-1535).
  - `finalize_stream` (1074-1102): if the stream returns `Ok(None)`, the caller falls back to a batch run over the buffered audio.
- **Handy's process isolation** (`Handy/src-tauri/src/engine_supervisor/`):
  - `protocol.rs:1-160`: the wire protocol is framed `[u32 json_len][json][u32 pcm_len][pcm f32le]` over stdin and stdout. JSON is used deliberately so NaN confidences survive the round trip. Requests are `Hello, Load, Run, StreamBegin, Feed, Finalize{want_language}, StreamReset`.
  - `worker.rs:32-165, 192-249, 255-325`: the child process.
    - It redirects fd 1 to stderr so stray ggml prints cannot corrupt the protocol, and reads stdin on a dedicated thread.
    - It `_exit`s on EOF, which skips ggml-metal's teardown asserts.
    - A stream loop runs inside the worker.
  - `supervisor.rs`: the parent side.
    - A single owner thread handles a command queue, so there is at most one worker.
    - Deadlines scale with audio length: on GPU, a 30 s floor or 10× the audio; on CPU, 120 s or 20× (50-107).
    - A crashed or hung worker is retried once on a fresh worker, on CPU if the GPU was at fault (860-896).
    - Cancel kills the worker (454-469).
    - Feeds never respawn the worker, because stream state would be lost. A lost stream finalizes to `None` (933-973, 975-1027).
    - `main.rs` checks `is_worker_invocation()` before Tauri starts (`Handy/src-tauri/src/main.rs:27-31`).
- **Handy's real-time-safe capture** (`audio_toolkit/audio/recorder.rs`):
  - The cpal callback only downmixes into an `rtrb` ring and counts overruns atomically. It does not allocate, lock or log (`build_stream` 426-483, `write_input_to_ring` 485-547).
  - A consumer thread resamples into VAD-sized 16 kHz frames and runs `handle_frame` (633-664), which applies VAD and calls the frame callback, which in turn calls `StreamRouter::feed`.
  - This is exactly the §3C fix the baseline audit asks for.
- **Handy's VAD** (`audio_toolkit/vad/`):
  - The `VoiceActivityDetector` trait (`mod.rs:37-60`) has Silero (30 ms frames) and Earshot (pure Rust, 16 ms frames) backends.
  - `SmoothedVad` (`smoothed.rs`) adds a 450 ms prefill, a 60 ms onset, and a hangover of 450 ms (offline) or 1650 ms (streaming) (`mod.rs:5-8`).

### Adopt for Deino

**Shape: per-lane workers, utterance-scoped streams, out-of-process engine.**

```
cpal / process-tap IO callback ──(rtrb SPSC, per channel)──► LaneWorker thread (one per Channel)
   resample once (rubato) → 16 kHz mono
   VAD (Earshot or Silero + SmoothedVad) → UtteranceSegmenter
   on speech start: SttStream::open  (stream begins at utterance_start: SampleIndex)
   every 160 ms of speech: stream.feed(chunk) → PartialText → UI event "transcript:partial"
   on speech end (hangover elapsed) or MAX_UTTERANCE: stream.finalize() → FinalUtterance
        words get absolute times: utterance_start + t0_ms
        → SegmentSink (FND-05 Rust-owned persistence, words_json) → UI "transcript:final"
EnginePool: one EngineSupervisor (worker process) per lane, each holding its own Model
```

Why utterance-scoped streams and not one stream per meeting:
- Memory stays bounded. `snapshot()` copies every token, so it must not grow for an hour.
- Segment boundaries come for free.
- Silence costs no GPU time.
- Timestamps stay exact, because each stream starts at a known capture sample. When a stream is gated by VAD for a whole meeting, its ms clock drifts away from wall-clock time (Handy feeds only VAD speech frames, which works for dictation and breaks meeting alignment).
- Cap utterances at `MAX_UTTERANCE` (about 30 s). Force the split at the next 300 ms dip in VAD probability.
- Feed the VAD prefill too, and record `utterance_start` as the prefill's first sample.

Why two worker processes: see finding 3. Two Parakeet Q8_0 models need about 1.46 GB of unified memory. On Macs with 8 GB or less, offer one of these:
- (a) Q4_K_M for both lanes (477 MB each).
- (b) A `TimeMultiplexedEngine`: one model, and each lane's utterance waits in a queue while the other lane's stream is active. Turn-taking makes overlaps rare, and the cost is one utterance of extra latency during double-talk.

Make the choice in the composition root from `sysctl hw.memsize`.

Trait and struct sketch, keeping §3A interface segregation (batch and streaming are separate traits):

```rust
//! audio/transcription/engine.rs: engine-neutral STT contracts.

/// Absolute position on a channel's 16 kHz capture timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleIndex(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Millis(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
pub enum Channel { Mic, System }

pub struct TimedWord { pub text: String, pub start: Millis, pub end: Millis, pub confidence: Option<f32> }
pub struct PartialText { pub committed: String, pub tentative: String, pub revision: u32 }
pub struct FinalTranscript { pub text: String, pub words: Vec<TimedWord>, pub timings: EngineTimings }

#[derive(Debug, thiserror::Error)]
pub enum SttError {
    #[error("model not loaded")] NotLoaded,
    #[error("engine busy")] Busy,
    #[error("engine worker lost: {0}")] WorkerLost(String),   // crash or hang; stream is gone
    #[error("cancelled")] Cancelled,
    #[error("engine error: {0}")] Engine(String),
}

/// Live, incremental decoding. One instance per utterance.
pub trait SttStream: Send {
    /// Feed 16 kHz mono PCM; returns new text only when it changed.
    fn feed(&mut self, pcm: &[f32]) -> Result<Option<PartialText>, SttError>;
    /// Flush and return final text with stream-relative word times.
    fn finalize(self: Box<Self>) -> Result<FinalTranscript, SttError>;
}

pub trait StreamingTranscriber: Send + Sync {
    fn open_stream(&self, params: &StreamParams) -> Result<Box<dyn SttStream>, SttError>;
}

/// Whole-buffer decoding (import, retranscribe, CAP-08 post-pass).
pub trait BatchTranscriber: Send + Sync {
    fn transcribe(&self, pcm: &[f32], params: &BatchParams) -> Result<FinalTranscript, SttError>;
}

pub struct StreamParams { pub language: Option<LanguageCode>, pub window: Option<ChunkWindow> }
pub struct ChunkWindow { pub left: Millis, pub chunk: Millis, pub right: Millis } // multiples of 80 ms
```

```rust
//! audio/transcription/lane.rs: one lane's worker loop (owns VAD + stream lifecycle).
pub struct LaneConfig { pub channel: Channel, pub feed_chunk: usize, pub max_utterance: Millis }
pub struct UtteranceEvent { pub channel: Channel, pub start: SampleIndex, pub kind: UtteranceEventKind }
pub enum UtteranceEventKind { Partial(PartialText), Final(FinalTranscript), Failed(SttError) }

pub struct LaneWorker<T: StreamingTranscriber, V: VoiceActivityDetector, S: UtteranceSink> {
    config: LaneConfig, transcriber: Arc<T>, vad: V, sink: S, state: LaneState,
}
enum LaneState { Idle, InSpeech { start: SampleIndex, stream: Box<dyn SttStream>, pending: Vec<f32> } }
pub trait UtteranceSink: Send { fn emit(&mut self, event: UtteranceEvent); }
```

```rust
//! audio/transcription/parakeet_unified_provider.rs: adapter over the supervised worker.
pub struct ParakeetUnifiedProvider { supervisor: EngineSupervisor /* copied from Handy */ }
impl StreamingTranscriber for ParakeetUnifiedProvider { /* StreamBegin; Feed; Finalize{want_snapshot: true} */ }
impl BatchTranscriber for ParakeetUnifiedProvider { /* Run with timestamps: TimestampKind::Word */ }
```

Threading model:
- **Audio IO thread:** ring write only.
- **One `LaneWorker` thread per channel:** blocking `feed` calls are fine here, because the round trip to the worker is a few ms for each 160 ms of audio.
- **One supervisor owner thread per worker process.**
- **Tokio** only for emitting UI events and persisting segments.

Handy's progress callback runs on the supervisor thread. Deino's lane thread can call synchronously instead, which is simpler to test with a `FakeTranscriber`.

Worker protocol changes compared with Handy:
- Add a `want_snapshot` flag to `Finalize`, and return `words` and `tokens`.
- Feed in 160 ms chunks, not 30 ms VAD frames. Handy pays one IPC round trip per 30 ms frame.
- Add `Request::Warmup` (see §2).

### Copy vs reimplement
- **Copy with attribution (MIT):**
  - `engine_supervisor/{protocol,worker,supervisor}.rs`. Split them to fit the 300-line, 40-line-function limits. Swap `anyhow` and `String` errors for `thiserror`.
  - The `recorder.rs` ring-write pattern.
  - `vad/{mod,smoothed}.rs`.
  - `StreamPerf` (`transcription.rs:1541-1649`).
  - Add Handy's notice to `THIRD_PARTY_NOTICES.md`, as CAP-01 says.
- **Reimplement:**
  - The `StreamRouter`/`TranscriptionManager` layer. It is dictation-shaped: one stream, a settings lookup inside the worker, Tauri `emit` inside the engine.
  - Language and Chinese-script logic is not needed (English only).

---

## 2. Model management

### Best found
**Handy, with one gap (finding 6).**
- **Bundled catalog.** `catalog/catalog.json` is compiled into the binary (`catalog/mod.rs:1-120`).
  - Each model lists `id` (HF repo), a pinned `revision`, `capabilities { streaming, timestamps: "token" }`, `files[] { filename, quant, size_bytes, sha256 }` and `default_quant`.
  - Parakeet Unified EN 0.6B is at `catalog.json:9-40` (revision `7e948f21b7bdbac698d3318db9d350f1096f3b6c`; full quant table in Appendix A).
  - `mirror_fallbacks` (`catalog/mod.rs:125-178`) builds `{mirror}/{repo}/{revision}/{filename}` URLs. It offers a mirror only when the catalog holds a sha256: "mirrors are untrusted bit-pipes; the sha256 is the trust anchor."
- **Resumable downloader:** `download_http_resumable_with_events` (`managers/model/download.rs:124-381`). Its strongest piece of engineering:
  - A partial file that is already full-size is verified, not re-requested (which would otherwise loop on a 416).
  - A 200 in reply to a Range request restarts from zero. A 206 must start exactly at our offset.
  - A server that sends more than the catalog size is cut off at the first excess byte.
  - Connect timeout is 15 s and stall timeout 60 s, and both race the cancel token.
  - Progress events are throttled to 10/s.
  - The sha256 is computed in 64 KB chunks on `spawn_blocking`, and the file is deleted on mismatch.
  - The transport is decoupled from Tauri through a callback, so it is tested against a local socket server (`download/tests.rs`, 579 lines).
- **HF orchestration** (`managers/model.rs:1922-2181`): retry schedule `[4, 1, 1, 1]` parallel streams with exponential backoff, a stall watchdog, and a mirror fallback.
- **Capability probing before download:**
  - `gguf_meta.rs` (494 lines) is a dependency-free GGUF header reader. It works on a partial buffer fetched with an HTTP Range request (`parse_header`, line 312).
  - The `CapabilityProber` trait is at `model_capabilities.rs:148`.
  - After load, the runtime capabilities from transcribe-cpp override the catalog (`transcription.rs:645-660`, `set_runtime_capabilities`).
- **Load lifecycle** (`transcription.rs:527-775`):
  - Emits `model-state-changed {loading_started|loading_completed|loading_failed|unloaded}`, with a terminal event guaranteed on every failure path.
  - Drops the old model before loading a new one, to avoid a memory peak.
  - `LoadingGuard` (198-221) is an RAII flag plus a condvar.
  - `initiate_model_load` (777-804) loads in the background at record start; `actions.rs:396-403` loads the ASR and VAD models in parallel.
- **Idle unload** (`transcription.rs:279-371`):
  - A watcher thread checks every 10 s. `ModelUnloadTimeout::{Never, Immediately, Min2, Min5(default), Min10, Min15, Hour1}` (`settings.rs:136-146`).
  - **It never unloads while recording**: it touches the activity timestamp instead.
  - Unloading means killing the worker, which returns all CPU and GPU memory (`request_unload` / `unload_model`, 449-469).
- **Warmup:** Handy does not run a warmup inference. It pre-warms device enumeration on a background thread at startup, because the first Metal listing compiles ggml's shader library (about 15 s after an install or update; `lib.rs:1028-1042`, `supervisor.rs:34-36` sets `HELLO_TIMEOUT` to 60 s for this).
- **Accelerator fallback:**
  - `Backend::Auto` plus `LoadedInfo.on_gpu` and `backend`.
  - After a GPU crash, the supervisor marks the GPU unavailable and reloads on a CPU-only worker that never registers GPU backends (`supervisor.rs:786-803`, `mod.rs:17-20`).

### Adopt for Deino
```rust
//! models/catalog.rs: bundled, typed model catalog (one JSON, compiled in).
pub struct ModelId(String);             // "handy-computer/parakeet-unified-en-0.6b-gguf/<file>"
pub struct Sha256Hex(String);
pub enum ModelKind { Stt, Vad, Segmentation, SpeakerEmbedding }
pub struct ModelFile { pub filename: String, pub quant: Quant, pub size_bytes: u64, pub sha256: Sha256Hex }
pub struct CatalogEntry { pub id: ModelId, pub kind: ModelKind, pub source: ModelSource, pub files: Vec<ModelFile>, pub default_quant: Quant, pub license: License }
pub enum ModelSource { HuggingFace { repo: String, revision: String }, GithubRelease { url: String } }

//! models/store.rs: on-disk state + downloads.
pub trait ModelDownloader: Send + Sync {        // adapter; fake in tests
    async fn fetch(&self, req: &DownloadRequest, progress: &dyn ProgressSink, cancel: &CancelToken) -> Result<DownloadOutcome, DownloadError>;
}
pub struct DownloadRequest { pub url: Url, pub partial: PathBuf, pub size: u64, pub sha256: Sha256Hex }
pub enum DownloadOutcome { Completed, Cancelled }
pub struct ModelStore<D: ModelDownloader> { root: PathBuf, downloader: D }
impl<D: ModelDownloader> ModelStore<D> {
    pub fn status(&self, id: &ModelId) -> ModelStatus;            // Missing | Partial{bytes} | Ready
    pub async fn ensure(&self, id: &ModelId, cancel: &CancelToken) -> Result<PathBuf, ModelError>;
}

//! models/lifecycle.rs: load/unload state machine per engine.
pub enum EngineState { Unloaded, Loading, Ready { backend: BoundBackend }, Unloading, Failed(SttError) }
pub struct UnloadPolicy { pub idle: Option<Duration>, pub never_while_recording: bool /* always true */ }
```

- **URLs:** download from `https://huggingface.co/{repo}/resolve/{revision}/{filename}` with Handy's resumable HTTP transport. Drop the `hf-hub` git fork. Verify size and sha256 always (finding 6).
  - Whether to use Handy's mirror (`https://blob.handy.computer`) is the owner's call. It is third-party infrastructure, and FND-12's data-egress rules apply. The default should be HF only.
- **One store for every model kind:** STT GGUF, VAD, segmentation and embedding ONNX. CAP-03's "download on demand with sha256 pins" then comes for free.
- **Warmup (Deino addition):** add a worker `Request::Warmup` that runs one stream over 1 s of zeros right after `Load`. This pays the Metal pipeline and kernel compile before the user's first word.
  - Start the load when CAP-06 shows the meeting prompt or at record start, whichever comes first.
  - Log `load_ms` (from `Timings.load_ms` or wall clock) and `warmup_ms`.
- **Quant selection:** default `Q8_0` (plan). On ≤8 GB RAM with two lanes, fall back to `Q4_K_M` (finding 3).
  - Read the bound backend from `LoadedInfo.backend`. If it is not `metal`, raise a health event (CAP-10), because the live lane will not keep up on CPU.

### Copy vs reimplement
- **Copy (MIT):**
  - `managers/model/download.rs` plus its test file, nearly verbatim. Split the 260-line function into prefix checks, request, body loop and verify.
  - `gguf_meta.rs`.
  - The catalog JSON entries and hashes for Parakeet Unified (all six quants) and the Whisper GGUFs if Whisper is kept.
- **Reimplement:**
  - `ModelManager` (3164 lines: HF-cache discovery, legacy migrations, custom-model scanning; none of it is needed).
  - The idle watcher, which should be built on Deino's `Clock` trait so it can be unit tested (§3A).

---

## 3. Speaker diarization

### Best found
**anarlog `crates/pyannote-local` (MIT).** Runner-up: sherpa-onnx's offline diarization.

**anarlog pyannote-local** (`anarlog/crates/pyannote-local/src/`):
- **`pipeline.rs:17-61` `DiarizationConfig`:**
  - Step 2 s over 10 s windows.
  - `min_embedding_seconds` 1.0 (clean speech needed before a window embedding becomes a clustering anchor).
  - `min_activity_seconds` 0.3.
  - Centroid-linkage cut 0.7045 (euclidean on unit vectors).
  - `min_cluster_seconds` 12.
  - `max_windows` 1800 (the step grows on long recordings so compute stays bounded).
  - `known_speaker_min_score` 0.62.
- **`pipeline.rs:105-136` `AudioSource`:** a random-access trait, so long recordings are read from a WAV/PCM file instead of held in memory.
- **`pipeline.rs:138-149` `DiarizeRequest`:** `bounds` (num/min/max speakers), `known_speakers: &[KnownSpeaker{id, embedding}]`, `is_cancelled`, `on_progress`.
- **`pipeline.rs:174-265` `diarize()`:** for each window, run segmentation (powerset over 3 local speakers, `segmentation.rs:1-100`), then embed each active local speaker with a **masked** WeSpeaker embedding (`embedding/src/onnx/mod.rs`, 256-d, fbank via `knf-rs`). Then:
  1. Cluster the anchors (`clustering.rs:69-260`: linkage, cut, centroids, nearest_centroid, min-cluster absorption).
  2. Apply known speakers (`pipeline.rs:335-448`).
  3. Reconstruct turns by majority vote (`450-585`).
  4. Renumber speakers by first appearance (`586-601`).
  - It returns `Diarization { segments: Vec<SpeakerSegment{start,end,speaker}>, speakers: Vec<DiarizedSpeaker{index, centroid, identity}> }`. **The centroids are exactly what CAP-03 must store.**
- **`pipeline.rs:603-636` `assign_words`:** each word goes to the speaker with the largest time overlap, falling back to the nearest turn when the word sits in a gap. This is the CAP-03 rule.
- **`metrics.rs`:** DER with an optimal one-to-one speaker mapping and a collar, matching pyannote.metrics conventions.
- **`eval/PROGRAM.md`:** an AMI and VoxConverse dev/test ratchet loop (DER at 0.25 s collar, keep a change only if DER improves by more than 0.1 points while RTF stays below 0.15). This is the documented, reproducible DER method that CAP-03's acceptance asks for.
- **`onnx` crate** (`anarlog/crates/onnx/src/lib.rs:15-110`): `ort =2.0.0-rc.10` with an optional CoreML execution provider, a compiled-model cache, and a fallback to CPU when the accelerator fails. Sessions run single-threaded (intra = 1).
- **Channel planning** (`anarlog/crates/listener2-core/src/batch/diarize.rs:20-52, 102-175`):
  - On a two-channel capture, the system channel is diarized with `speaker budget − 1`.
  - The mic channel is diarized **only** for in-person sessions: several participants expected **and** the system channel carried no more than `REMOTE_SILENCE_MAX_SECONDS` = 3 s of speech.
  - A shared mic gets its own `SHARED_MIC_CHANNEL` so it is not all folded into "You".
  - A progress heartbeat is sent every 5 s.

**sherpa-onnx** (`sherpa-onnx/sherpa-onnx/csrc/offline-speaker-diarization-pyannote-impl.h`, 891 lines):
- **Algorithm:** pyannote segmentation-3.0 over sliding windows (`window_shift_ratio` 0.1, so a 1 s hop; 297-357), powerset to multi-label conversion, speakers per frame, one embedding per (chunk, local speaker) with NaN filtering (157-195), then `FastClustering`.
  - `FastClustering` (`fast-clustering.cc`) is fastcluster complete-linkage on cosine distance, cut with `cutree_k` (`num_clusters` > 0) or `cutree_cdist` at `threshold`, default 0.5.
  - Defaults: `min_duration_on` 0.3, `min_duration_off` 0.5.
- **Rust API:** official crate `sherpa-onnx` 1.13.8 (`sherpa-onnx/sherpa-onnx/rust/sherpa-onnx/src/offline_speaker_diarization.rs`).
  - `OfflineSpeakerDiarizationConfig { segmentation: {pyannote: {model, window_shift_ratio}, num_threads, provider}, embedding: SpeakerEmbeddingExtractorConfig, clustering: FastClusteringConfig { num_clusters: -1, threshold: 0.5 }, min_duration_on, min_duration_off }`.
  - `OfflineSpeakerDiarization::create(&cfg)`, then `.process(&samples) -> Result.sort_by_start_time() -> Vec<{start, end, speaker, confidence}>`.
  - The handle is `Send + Sync`. **No centroids are exposed.** The C API has `...ProcessWithCallback` for progress (`c-api/c-api.h:4116`), but the safe wrapper does not expose it.
  - `build.rs` downloads prebuilt static or shared libraries from GitHub Releases at build time; `SHERPA_ONNX_LIB_DIR` and `SHERPA_ONNX_ARCHIVE_DIR` are the overrides for offline builds.
  - `cargo search sherpa`: official `sherpa-onnx = 1.13.8` / `sherpa-onnx-sys`; third-party `sherpa-rs = 0.6.8` (MIT, thewh1teagle; features `download-binaries`, `static`, `tts`).
- **Online vs offline:** sherpa-onnx has **no online diarization**. It has only the offline pipeline plus speaker-identification examples. `python-api-examples/speaker-identification-with-vad-dynamic.py:160-211` is a leader-follower loop: embed each VAD segment, `manager.search(emb, threshold=0.4)`, or register `speaker_N`.

**openwhispr** shows the tuning lessons that matter in practice (`openwhispr/src/helpers/diarizationPolicy.js:1-60`). It runs sherpa-onnx diarization through a sidecar CLI (`diarization.js:287-345`) with CAM++ 512-d embeddings.
- **The clustering threshold should grow with recording length.** It ramps from 0.55 at 15 minutes or less to 0.70 at 60 minutes, because a 73-minute voice memo produced 46 "speakers". It is capped below 0.72, because at that level two speakers merged into one.
- **Phantom clusters are pruned:** under 1 s of total speech, or under 10% of the speech with a mean segment under 3 s. The two largest clusters are never pruned.
- **A collapsed cluster is detected:** the top cluster holds 93% or more of the speech and the others average 2.2 s or less.

**Live (best-effort) clustering.** openwhispr `liveSpeakerIdentifier.js`:
- Thresholds (33-50): 1.5 s minimum segment, `MATCH_THRESHOLD` 0.65, `MATCH_MARGIN` 0.03.
- Matching (685-747): the best match must clear **both** the absolute threshold **and** the margin over the runner-up.
- Resolution (749-808): try existing in-meeting speakers first, then stored profiles.
- Cap handling (810-822): at the speaker cap, a voice that matches no cluster is labelled with the nearest one but **never folded into that centroid**.
- IDs (857-879): IDs are never reused after a merge, and centroids are a running mean.

### Adopt for Deino
Post-meeting stage `diarize` (FND-08 job, order 30):

```rust
//! diarization/mod.rs: speaker diarization (who spoke when) for one recorded track.
pub struct SpeakerTurn { pub start: Millis, pub end: Millis, pub speaker: LocalSpeaker }
#[derive(Clone, Copy, PartialEq, Eq, Hash)] pub struct LocalSpeaker(pub u16);
pub struct Embedding(pub Vec<f32>);                     // unit-normalised; dim fixed per model
pub struct EmbeddingModelId(pub String);                // e.g. "wespeaker-resnet34-lm@<sha256>"
pub struct DiarizedSpeaker { pub local: LocalSpeaker, pub centroid: Embedding, pub speech: Millis }
pub struct Diarization { pub turns: Vec<SpeakerTurn>, pub speakers: Vec<DiarizedSpeaker>, pub model: EmbeddingModelId }
pub struct SpeakerBounds { pub exact: Option<u16>, pub min: Option<u16>, pub max: Option<u16> }

pub trait AudioSource: Send { fn len(&mut self) -> Result<usize, DiarizeError>; fn read(&mut self, start: usize, out: &mut [f32]) -> Result<(), DiarizeError>; }
pub trait Diarizer: Send {
    fn diarize(&mut self, audio: &mut dyn AudioSource, request: &DiarizeRequest<'_>) -> Result<Diarization, DiarizeError>;
}
pub struct DiarizeRequest<'a> { pub bounds: SpeakerBounds, pub known: &'a [KnownVoice], pub cancel: &'a CancelToken, pub progress: &'a dyn Fn(f32) }

pub trait SpeakerEmbedder: Send {                       // shared with CAP-04 and live clustering
    fn model_id(&self) -> &EmbeddingModelId;
    fn embed(&mut self, pcm16k: &[f32]) -> Result<Option<Embedding>, DiarizeError>; // None when too short
}

//! diarization/assign.rs: pure functions (unit-tested).
pub fn assign_words(words: &[TimedWord], turns: &[SpeakerTurn]) -> Vec<Option<LocalSpeaker>>;
pub fn split_segments_at_speaker_changes(segment: &Segment, labels: &[Option<LocalSpeaker>]) -> Vec<Segment>;
```

Stage flow:
1. Open `system.m4a` (from FND-05) through a decoding `AudioSource`, at 16 kHz mono.
2. Bounds:
   - Use the calendar attendee count minus one when it is known (anarlog `expected_speaker_cap`).
   - Otherwise leave `num_speakers` unset and let the threshold decide.
   - Ramp the threshold by duration (openwhispr).
3. Run `Diarizer::diarize`, passing **known voiceprints of the attendees** (CAP-04) so fragmented clusters merge.
4. Diarize the mic track only when "In-person meeting" is on or auto-detected. Auto-detection means the system track's VAD speech is at most 20% of the meeting (plan) **or** at most 3 s in total (anarlog), and the attendee count is at least 2.
5. Assign each word by overlap, split segments at speaker changes, then write `speakers` rows (`label = "Speaker N"`, `centroid BLOB`, `embedding_model`, `speech_ms`) in one transaction.
6. Prune phantoms (openwhispr rules) before writing.

Live (best-effort) path:
- Implement a `LiveSpeakerTracker` on the system lane. On each `FinalUtterance` of at least 1.5 s, embed it with the same `SpeakerEmbedder` on a background thread (never the lane thread) and match it with the threshold-plus-margin rule.
- Update a centroid only on a confident match. Never reuse IDs.
- The post-meeting pass replaces the live labels: map live speakers to post-meeting speakers by majority word overlap so any renames the user made live carry over.

Performance against the CAP-03 target (60 minutes in under 2 minutes on an M1, i.e. RTF ≤ 0.033):
- anarlog's ratchet allows RTF ≤ 0.15. That is not good enough here, so budget for it:
  - Use more intra-op threads than anarlog's single thread, or the CoreML provider for the embedding model.
  - Use a step of 3–4 s for meetings over 30 minutes.
  - Batch embedding inference per window.
- Measure with the eval binary on the reference Mac before tuning DER.

### Copy vs reimplement
- **Port (MIT, with attribution):**
  - `anarlog/crates/pyannote-local/src/{pipeline,clustering,segmentation,metrics}.rs`.
  - `anarlog/crates/embedding/src/onnx/*`.
  - `anarlog/crates/onnx/src/lib.rs`.
  - Adapt them to Deino's error types, newtypes and size limits. `pipeline.rs` is 600+ lines and must be split into `windowing`, `known_speakers` and `reconstruct`.
  - Replace the git dependency `knf-rs` (thewh1teagle/pyannote-rs@e23bd29, which wraps kaldi-native-fbank C++) with a vendored copy or a crates.io fbank.
  - **Do not `include_bytes!` the ONNX files** as anarlog does. Fetch them through §2's store with sha256 pins, as CAP-03 says.
- **Reimplement:** the live tracker (port openwhispr's 200 lines of JavaScript logic to Rust), the phantom and threshold policy, and the stage wiring.
- **If sherpa-onnx is chosen instead:** use the official `sherpa-onnx` crate with `features = ["shared"]` and `SHERPA_ONNX_LIB_DIR` pointing at a pinned, hash-checked archive.
  - Check for duplicate onnxruntime symbols against `ort` in a test link.
  - Compute centroids separately with `SpeakerEmbeddingExtractor` over each speaker's selected spans (see §4).

---

## 4. Voiceprints (recognising people across meetings)

### Best found
**anarlog `crates/voiceprint` (pure Rust, no dependencies, MIT), plus its exemplar storage design.**
- **`voiceprint/src/lib.rs:65-185` `select_speaker_spans`:**
  - Merges each speaker's words into runs (gap ≤ 400 ms).
  - **Cuts out any portion that overlaps another speaker on the same channel**, because overlapped speech contaminates embeddings.
  - Keeps spans of at least 1.5 s, clipping long ones to their middle 10 s, since edges carry hesitation noise.
  - Keeps the 3 longest per speaker, with `quality = duration / max_span`.
- **`voiceprint/src/matching.rs`:**
  - `MIN_UNIQUE_SCORE` 0.62 and `MIN_UNIQUE_MARGIN` 0.08 (3-4).
  - `pick_unique_voiceprint_assignments` (41-105) is **mutual best match**: speaker S gets person P only if P is S's unique best above the floor and margin, *and* S is P's unique best.
  - `complete_by_elimination` (143-163): when the number of speakers on a channel equals the number of possible people and all but one pair are assigned, the last speaker is the last person.
  - `collect_match_scores` (189-215) compares only exemplars from the **same capture domain**.
- **Storage** (`anarlog/crates/db-app/migrations/20260802120000_voiceprint_exemplars.sql`, `…20260903120000_voiceprint_exemplars_isolated_mic.sql`):
  - Exemplar columns: `human_id`, `model_provider`, `model_version`, `capture_domain` (`direct_mic` | `system_audio` | `mixed_capture`), `confirmation_source` (`manual_speaker_assignment` | `accessibility_active_speaker` | `isolated_mic_capture`), the source session and span, `quality_score`, `label_confidence`, and soft delete.
  - The embedding vector itself lives in the OS keyring (`keyring_scope`/`keyring_key`), because it is biometric data.
  - Candidates expire after 45 days (`plugins/transcription/src/voiceprint.rs:13-23`).
- **Owner voiceprint without user action** (`plugins/transcription/src/voiceprint.rs:414-530`): with headphone output, the direct-mic channel can only be the user, so those spans confirm the owner's voiceprint at label confidence 0.9.
- **openwhispr storage** (`openwhispr/src/helpers/database.js:969-1000, 5895-5915, 5977-6010`) is the simpler pattern and the weaker one:
  - It keeps one `speaker_profiles.embedding` per person, updated with an EMA (0.3 new, 0.7 old).
  - It merges profiles by a sample-count-weighted average.
  - **Avoid this**: one drifting vector per person has no model or domain tag and no per-exemplar provenance, and it cannot drop a bad sample.

### Adopt for Deino
Keep CAP-04's "up to 5 voiceprints per person", but store them as anarlog-style exemplars:

```sql
CREATE TABLE voiceprints (
  id               TEXT PRIMARY KEY NOT NULL,
  person_id        TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
  embedding        BLOB NOT NULL,                 -- f32 LE, unit-normalised
  embedding_model  TEXT NOT NULL,                 -- EmbeddingModelId; vectors from different models never compared
  capture_domain   TEXT NOT NULL CHECK (capture_domain IN ('mic','system')),
  source           TEXT NOT NULL CHECK (source IN ('rename_all','isolated_mic','speaker_tag')),
  source_meeting_id TEXT REFERENCES meetings(id) ON DELETE SET NULL,
  source_start_ms  INTEGER NOT NULL, source_end_ms INTEGER NOT NULL CHECK (source_end_ms > source_start_ms),
  quality          REAL NOT NULL CHECK (quality BETWEEN 0 AND 1),
  created_at       TEXT NOT NULL
) STRICT;
CREATE INDEX idx_voiceprints_person ON voiceprints(person_id, created_at);
```

```rust
//! diarization/identify.rs: stage `identify_speakers` (order 35). Pure matching core + thin stage.
pub struct Voiceprint { pub person: PersonId, pub embedding: Embedding, pub model: EmbeddingModelId, pub domain: CaptureDomain, pub quality: f32 }
pub struct MatchThresholds { pub auto_assign: f32, pub suggest: f32, pub margin: f32 }   // tunable; calibrate per model
pub enum Identification { Assigned { person: PersonId, score: f32 }, Suggested(Vec<(PersonId, f32)>), Unknown }
pub fn identify(speakers: &[DiarizedSpeaker], prints: &[Voiceprint], attendees: &[PersonId], t: &MatchThresholds) -> Vec<Identification>;
pub fn select_exemplar_spans(words: &[SpanWord], config: &SpanConfig) -> Vec<SelectedSpan>;   // port of anarlog lib.rs
pub trait VoiceprintRepo { fn for_people(&self, people: &[PersonId], model: &EmbeddingModelId) -> Result<Vec<Voiceprint>, RepoError>;
                           fn insert_capped(&self, print: &NewVoiceprint, max_per_person: usize) -> Result<(), RepoError>; }
```

Rules:
- **Score.** A speaker's score for a person is the **maximum** cosine similarity over that person's exemplars (not a mean), as in anarlog's `collect_match_scores`.
- **Auto-assign.** Require mutual best, plus the absolute threshold, plus the margin. Attendees are tried first; non-attendees need `auto_assign + 0.05`. After that, complete by elimination when the speaker and attendee counts match exactly.
- **Thresholds are specific to the embedding model.** The plan's 0.75 auto / 0.6 suggest values are a starting point. anarlog uses 0.62 with a 0.08 margin for WeSpeaker ResNet34; openwhispr uses 0.65 with a 0.03 margin for CAM++. Calibrate on the CAP-04 fixtures and store the thresholds in typed settings.
- **"The mic speaker is always Me."** In addition, when the "I use headphones" setting is on (CAP-02), insert `isolated_mic` exemplars for the owner automatically from the mic lane. Do not use the system lane for this.
- **On "rename all":** pick that speaker's three best spans with `select_exemplar_spans`, embed each one, and insert them. Evict the lowest-`quality` exemplar beyond 5 per person, in the same transaction.
- **Privacy:** voiceprints are biometric data. Delete them when the person is deleted (`ON DELETE CASCADE`). Settings → People → Voice profiles lists them and deletes them. Never log embeddings. Encrypting the column with a Keychain-held key (anarlog keeps the vector in the keyring) is a reasonable FND-06 follow-up.

### Copy vs reimplement
- **Copy (MIT):** `anarlog/crates/voiceprint/src/{lib,matching}.rs` almost verbatim (661 lines including tests). Swap the `String` human IDs for `PersonId`.
- **Reimplement:** the stage, the repository, and the eviction policy. Do not copy openwhispr's EMA approach.

---

## 5. Separate channels, and echo and bleed handling

### Best found
Three layers, from three repos:
- **anarlog: signal-level alignment and AEC** (`anarlog/crates/audio-actual/src/capture/stream.rs`):
  - A `Joiner` pairs mic and speaker chunks (`capture/joiner.rs`).
  - `AecReferenceAligner` (212-405) uses a **GCC-PHAT lag estimator** with drift tracking (`anarlog/crates/audio-sync/src/estimator.rs:7-60`, `probe.rs`). Max lag 600 ms, minimum overlap 400 ms, minimum correlation 0.12 (26-35). It delays whichever side leads, and resets the AEC whenever the alignment changes.
  - **DTLN-aec** (two-stage ONNX, 128/256/512 state variants) then processes the aligned pair (`anarlog/crates/aec/src/onnx/mod.rs:53-300`).
  - A residual linear echo-gain cancellation follows (`stream.rs:522-600`).
  - Both raw and AEC'd mic are kept (`anarlog/crates/audio/src/lib.rs:39-60`, `CaptureFrame { raw_mic, raw_speaker, aec_mic }`).
  - The VAD **masks** non-speech mic samples to zero instead of dropping them, which keeps the timeline intact (`anarlog/crates/vad-masking/src/streaming.rs`; `listener-core/src/actors/source/pipeline.rs:136-160, 255-269`).
  - anarlog has no text-level dedupe.
- **openwhispr: segment-level duplicate suppression.** This is the most thorough and the most battle-tested:
  - `meetingMicGate.js:1-57`: a per-chunk gate. Silent means RMS < 0.0015 and peak < 0.05. Quiet means RMS < 0.018 and peak < 0.07. A quiet mic while the system is speaking is zeroed (`bleed_floor`).
  - `meetingEchoLeakDetector.js:7-37, 83-345`: mic-to-system correlation over a ±500 ms lag scan, plus residual and explained-energy ratios. It classifies chunks as strong bleed, partial bleed or double-talk.
  - `meetingMicHoldback.js:1-110`: the policy. **A text match is the only thing that may drop a segment. Audio evidence only delays it.**
    - A "risky" mic final (system speaking, bleed or double-talk flagged, or the startup warmup) is held back for a window.
    - It is then dropped if a system transcript nearby overlaps it.
    - A segment already released can still be retracted when a matching system final arrives late.
  - `transcriptText.js:1-205`: overlap tests.
    - Strict: normalized tokens, containment, or token coverage of at least 0.6, or token LCS of at least 0.6, needing at least 3 tokens.
    - Loose (for double-talk): the same after dropping stop words, with 0.55 thresholds and at least 4 meaningful tokens.
  - `diarization.js:28-52`: the same dedupe in the post-meeting path, against merged system segments within ±6 s.
- **Handy:** single channel (dictation). Nothing to reuse here.

### Adopt for Deino
Keep the lanes separate from capture to storage (§3C). Do not run VAD or STT on the mix.

```rust
//! audio/bleed/mod.rs: mic-vs-system duplicate suppression (CAP-02). Pure, clock-injected.
pub struct ChunkStats { pub rms: f32, pub peak: f32 }
pub enum MicChunkVerdict { Keep, ZeroAsBleed }          // zero, never drop: keeps the timeline
pub fn gate_mic_chunk(mic: ChunkStats, system_speaking: bool, t: &GateThresholds) -> MicChunkVerdict;

pub enum BleedEvidence { None, SystemSpeaking, LikelyBleed, DoubleTalk }
pub struct HeldSegment { pub segment: FinalUtterance, pub evidence: BleedEvidence, pub release_at: Millis }
pub enum Decision { Release(FinalUtterance), DropAsDuplicate { of: SegmentId }, Hold }

pub trait TextOverlap { fn overlaps(&self, a: &str, b: &str, mode: OverlapMode) -> bool; }   // strict | loose
pub struct BleedSuppressor<O: TextOverlap, C: Clock> { overlap: O, clock: C, held: Vec<HeldSegment>, window: Millis }
impl<O: TextOverlap, C: Clock> BleedSuppressor<O, C> {
    pub fn on_mic_final(&mut self, utterance: FinalUtterance, evidence: BleedEvidence) -> Vec<Decision>;
    pub fn on_system_final(&mut self, utterance: &FinalUtterance) -> Vec<Decision>;   // may retract a released mic segment
    pub fn tick(&mut self) -> Vec<Decision>;                                          // releases expired holds
}
```

- **Data flow:**
  1. The mic lane computes `ChunkStats` per 160 ms and asks the system lane's VAD state, through a shared `AtomicBool` plus the last speech `SampleIndex`, whether the system is speaking.
  2. It zeroes bleed-floor chunks *before* the VAD and STT.
  3. Every mic `FinalUtterance` goes through `BleedSuppressor` before persistence.
  4. Every system final is offered to it as a retraction candidate.
  5. A retraction becomes a soft delete in the segment repository plus a UI event.
- **"I use headphones" setting:** turn off steps 2–5 entirely (plan). In addition, use the mic to auto-collect owner voiceprints (§4).
- **Text measure:** keep the plan's "normalized Levenshtein ratio > 0.6", but compute it on **tokens**, with openwhispr's coverage-or-LCS rule. Character Levenshtein misfires on short utterances, so require at least 3 tokens. Use a ±6 s window and a holdback of about 2 s. Holding a segment back delays its *final* rendering only; partial text can still show, greyed out.
- **Clocks:** every comparison uses the shared capture `SampleIndex` timeline, not `Date.now()` as openwhispr does. That timeline is also the precondition for the CAP-03 overlap logic.
- **AEC is out of scope for CAP-02 v1.**
  - The text-level path meets the "<5% duplicated lines" acceptance criterion at almost no CPU cost.
  - If the fixture shows otherwise, add anarlog's GCC-PHAT aligner and DTLN-aec behind a setting. Its cost is about one extra ONNX model on the mic lane.
  - Apple's Voice Processing I/O (`kAudioUnitSubType_VoiceProcessingIO`) is another option, but none of the reference repos uses it. It also changes mic gain and ducks other audio, so it would need its own ADR.

### Copy vs reimplement
- **Reimplement in Rust,** porting openwhispr's thresholds and rules (`meetingMicGate.js`, `meetingMicHoldback.js`, `transcriptText.js`). They are small, pure and table-testable. Port openwhispr's characterization tests as fixtures.
- **Optional later:** copy anarlog's `audio-sync` crate (GCC-PHAT, drift) and `aec` (DTLN, MIT, with model files) under attribution.

---

## 6. Performance measurement

### Best found
- **Handy `StreamPerf`** (`managers/transcription.rs:1541-1649`, logged every 5 s by `STREAM_PERF_LOG_INTERVAL`, line 41). Fields:
  - streamed audio seconds and model compute seconds, which give the RTF;
  - `input_received_ms`, `audio_committed_ms`, `buffered_ms` and `revision` from `StreamUpdate`;
  - frames fed and updates emitted;
  - a final line per stream.
- **Handy batch RTF log:** `transcription.rs:1226-1238`.
- **`Transcript.timings { load_ms, mel_ms, encode_ms, decode_ms }`:** `TC/src/result.rs`.
- **Supervisor deadlines:** these double as performance guards. A feed that runs past `10 × audio` on the GPU counts as a hang (`supervisor.rs:50-107`). `pending_ms = max(buffered_ms, input_received_ms − audio_committed_ms)` (1050-1055) measures how far decoding lags behind input. That is the live "backlog" metric.
- **anarlog:** DER and RTF via `pyannote-local/src/metrics.rs` and `eval/PROGRAM.md`, plus `criterion` benches for the AEC and VAD (`anarlog/crates/aec/benches`, `crates/vad`).
- **Handy recorder:** atomic overrun counters (`recorder.rs:857-870`) and first-sample latency logs (809-850).

### Adopt for Deino
```rust
//! audio/transcription/metrics.rs: per-lane STT metrics; logs IDs and numbers only (§3A).
pub struct LaneMetrics {
    pub channel: Channel,
    pub audio_fed: Millis, pub compute: Duration,             // RTF = audio_fed / compute
    pub first_text_latency: Histogram,                        // speech onset (VAD) -> first non-empty partial
    pub final_latency: Histogram,                             // speech end -> FinalUtterance
    pub backlog_max: Millis,                                  // max pending_ms
    pub ring_overruns: u64, pub worker_restarts: u32, pub fell_back_to_cpu: bool,
}
pub trait MetricsSink: Send { fn record_utterance(&mut self, m: &UtteranceMetrics); fn flush_meeting(&mut self, meeting: MeetingId); }
```

- **Log RTF** per utterance at debug level and per meeting at info level (CAP-01 acceptance). Log `first_text_latency` p50 and p95 per meeting; this is the honest test of "≈1.5 s", and its 1.5 s target sits at p50.
- **CPU acceptance (CAP-02 ≤ 1.6× single-lane, CAP-08 < 15%) must add up the app process and every STT worker process.** Use `proc_pid_rusage` or `task_info` per PID; the supervisor knows the worker PIDs (`Control.pid`). Sample every 5 s in a debug-only `PerfProbe` and log the totals at meeting end.
- **Diarization:** log `diarize_rtf`, `windows`, `speakers_found` and `phantoms_pruned` per stage run. Add `cargo run --bin diarization-eval` (ported from anarlog) for DER on the fixtures (CAP-03 acceptance), and record results in `docs/perf/diarization.tsv`.
- **Benchmarks:** `criterion` benches for the VAD, the resampler and `BleedSuppressor`. A manual `stt-bench` binary that streams a fixture WAV at 1× through the real worker and prints the latency histograms.

### Copy vs reimplement
- **Copy:** `StreamPerf`, `real_time_factor` and `pending_ms` (Handy), and `metrics.rs` (DER) from anarlog.
- **Reimplement:** the sink and the per-process CPU probe (macOS-specific, not in any of the reference repos).

---

## Recommendations by card

### CAP-01 · Parakeet Unified EN 0.6B engine
- **Read first:**
  - `Handy/src-tauri/src/engine_supervisor/{mod,protocol,worker,supervisor}.rs`
  - `Handy/src-tauri/src/managers/transcription.rs:125-183, 527-775, 844-1102, 1541-1649`
  - `Handy/src-tauri/src/managers/model/download.rs:124-381`
  - `Handy/src-tauri/src/catalog/{mod.rs,catalog.json:9-40}`
  - `TC/src/{lib,session,streaming,result,family}.rs`
  - `TCS/include/transcribe/parakeet.h:70-106`
- **Dependency:** pin `transcribe-cpp = { version = "=0.3.1", default-features = false, features = ["metal"] }` on macOS, not 0.1.3. Remove `whisper-rs` and move Whisper to transcribe-cpp GGUFs, or prove with a link test that the two ggml copies do not collide (finding 7).
- **Process isolation:** run the engine out of process with Handy's supervisor (`deino --stt-worker`), and check for the flag in `main.rs` before Tauri starts. Extend the protocol so `Finalize` returns `snapshot()` words and tokens, which go into `words_json`. Handy's protocol returns text only.
- **Streaming:** use utterance-scoped streams per lane, fed 160 ms chunks, with absolute word times = `utterance_start + t0_ms`. Do not gate a long-lived stream with VAD; the timestamps would drift.
- **Latency:** the default (5600/1040/1040 ms) window gives about 2.1 s to first text. Read the menu from the GGUF (`stt.parakeet.encoder.att_chunk_*_choices`, via the copied `gguf_meta.rs`), choose the smallest (C, R) whose accuracy on the fixtures is acceptable, and log p50 and p95 first-text latency.
- **Downloads:** catalog plus resumable HF `resolve/<revision>` download, with sha256 verification **every time**. Handy skips it on the HF path. Keep all six quant hashes (Appendix A).
- **Startup:** warm up after load (1 s of zeros through a stream) and pre-warm device enumeration in the background at startup. The first Metal shader compile takes about 15 s, so the hello timeout must be at least 60 s.
- **Unloading and fallback:**
  - Never unload while recording. The idle unload kills the worker.
  - Surface `LoadedInfo.backend != "metal"` as a health warning.
  - On a worker crash during a stream, the lane finalizes to `None`. Re-transcribe the buffered utterance with a batch `run` on the restarted worker, and never lose audio.
- **Pitfalls:**
  - `RunOptions::vocabulary` is ignored by Parakeet; use `apply_custom_words`-style fuzzy correction.
  - Import and retranscribe must use the same provider's `BatchTranscriber`, chunked by VAD to at most `capabilities.max_audio_ms`.

### CAP-02 · Me/Them channel separation
- **Read first:**
  - `references/audio.md` (mixer and VAD-on-mix problems)
  - `Handy/src-tauri/src/audio_toolkit/audio/recorder.rs:426-664`
  - `openwhispr/src/helpers/{meetingMicGate,meetingMicHoldback,meetingEchoLeakDetector,transcriptText}.js`
  - `anarlog/crates/listener-core/src/actors/source/pipeline.rs:136-269`
  - `anarlog/crates/vad-masking/src/streaming.rs`
- **Lanes:** two `LaneWorker`s, one per `Channel`, each with its own VAD, `SttStream` and **its own engine worker process**. One model allows one stream at a time (finding 3).
  - On ≤8 GB Macs, use Q4_K_M or the time-multiplexed engine.
  - Budget about 1.5 GB of unified memory for two Q8_0 models.
- **Timeline:** both lanes share one capture timeline (`SampleIndex` at 16 kHz since recording start). Zero-pad and **mask**, never drop, so mic and system stay aligned for dedupe, diarization and playback.
- **Suppression:** gate mic chunks by energy (zero when quiet and the system is speaking), then apply segment-level `BleedSuppressor`: hold back, then drop **only on a text overlap** (token coverage or LCS ≥ 0.6 within ±6 s), with late retraction. **Pitfall:** do not drop on audio correlation alone (finding 10).
- **"I use headphones":** bypass all suppression, and feed the owner's `isolated_mic` voiceprint collection.
- **Default speakers:** mic → the "Me" speaker (`is_me = 1`), system → "Them". Write `channel` on every segment from the lane, not from the device type. Today everything is tagged `Microphone`.
- **CPU acceptance (≤ 1.6× single lane):** measure app plus worker PIDs. Most compute is on the Metal GPU, so also log GPU time per stream (`StreamPerf.compute`).
- **Fixture:** a 10-minute laptop-speaker recording with a hand-labelled duplicate list, so the "<5% duplicated lines" check is a deterministic test of the pure `BleedSuppressor` on recorded `FinalUtterance`s plus one end-to-end run.

### CAP-03 · Local speaker diarization
- **Read first:**
  - `anarlog/crates/pyannote-local/src/{pipeline.rs:17-265,335-636, clustering.rs, metrics.rs}`
  - `anarlog/crates/pyannote-local/eval/PROGRAM.md`
  - `anarlog/crates/listener2-core/src/batch/diarize.rs:20-175`
  - `openwhispr/src/helpers/diarizationPolicy.js:1-60`
  - `openwhispr/src/helpers/liveSpeakerIdentifier.js:685-879`
  - `sherpa-onnx/sherpa-onnx/csrc/offline-speaker-diarization-pyannote-impl.h`
- **Engine choice:** port anarlog's pyannote-local (MIT, same `ort` version) instead of adding `sherpa-rs` (finding 8). Write an ADR, because this changes the plan text. If sherpa-onnx is kept, use the official `sherpa-onnx` 1.13.8 crate, and compute centroids yourself.
- **Models:** pyannote segmentation-3.0 ONNX (MIT) plus WeSpeaker ResNet34-LM (256-d, CC-BY-4.0, needs attribution), downloaded through the CAP-01 model store.
  - Pin sha256 values you compute yourself at implementation time. GitHub release assets publish no digest.
  - **Pitfall:** `pyannote/segmentation-3.0` on HF is gated. Use the sherpa-onnx release asset or a pinned HF mirror.
  - **Pitfall:** avoid `sherpa-onnx-reverb-diarization-v1/v2` (Rev.ai non-commercial licence).
- **Inputs:**
  - System track always. Mic track only for in-person meetings: the toggle, or auto when system speech is at most 20% (or at most 3 s) and there are at least 2 attendees.
  - Bounds from the attendee count minus one.
  - Known voiceprints of the attendees passed in, so fragmented clusters merge.
- **Post-processing:**
  - Ramp the threshold with duration (openwhispr: 0.55 → 0.70 between 15 and 60 minutes, on its cosine scale; re-derive for anarlog's euclidean-centroid cut).
  - Prune phantom clusters (< 1 s, or < 10% share with a mean turn < 3 s). Never prune the two largest.
- **Assigning words:** assign by maximum overlap, falling back to the nearest turn (`assign_words`), and split segments where the speaker changes mid-segment using word times from CAP-01. Write `speakers.centroid` and `embedding_model` for CAP-04.
- **Live clustering:** embed finalized system utterances of at least 1.5 s off the lane thread, match with threshold plus margin, never pollute centroids on forced matches, never reuse IDs. The post-meeting pass replaces the result, with live renames mapped by word overlap.
- **Acceptance:**
  - DER: port `metrics.rs`, use a 0.25 s collar, and document it in `docs/perf/diarization.md` (two 3–4-speaker fixtures).
  - Speed (60 minutes in under 2 minutes means RTF ≤ 0.033): this is tighter than anarlog's own 0.15 budget. Plan for multi-threaded or CoreML embedding, a 3–4 s step on long meetings, and check on an M1 early.

### CAP-04 · Speaker naming and voice profiles
- **Read first:**
  - `anarlog/crates/voiceprint/src/{lib.rs:65-185, matching.rs}`
  - `anarlog/crates/db-app/migrations/2026080212*_voiceprint_exemplars.sql`, `…20260903…isolated_mic.sql`
  - `anarlog/plugins/transcription/src/voiceprint.rs:13-23, 414-530, 822-1000`
  - openwhispr `database.js:969-1000, 5977-6010`, as the pattern to avoid
- **Storage:** exemplars, not one averaged vector. Each row carries `embedding_model`, `capture_domain` (mic or system), source span and `quality`; keep at most 5 per person and evict the lowest quality. Only compare vectors from the same model and the same domain.
- **Spans for exemplars:** use `select_speaker_spans`: overlap-free spans of at least 1.5 s, clipped to their middle 10 s, top 3 per speaker. Embed spans, not the diarizer's centroid alone; the plan's "speaker centroid" can be one exemplar among them.
- **Matching:** take the maximum over a person's exemplars, require a **mutual** best plus the threshold plus a margin, prefer calendar attendees (stricter threshold for non-attendees), then complete by elimination when counts match exactly. Mic is always Me.
- **Thresholds:** 0.75/0.6 are starting values. **Calibrate per embedding model on fixtures** (anarlog 0.62 + 0.08 margin; openwhispr 0.65 + 0.03), and store them in typed settings.
- **Owner voiceprints:** with "I use headphones" on, collect the owner's `isolated_mic` exemplars automatically, so "Me" gets a voiceprint without any rename.
- **Rename and merge:**
  - Rename-all writes the person, the voiceprints and the segment updates in one transaction, then invalidates note-generation inputs.
  - Merging two speakers concatenates their spans and recomputes the centroid. Do not use openwhispr's weighted EMA.
- **Privacy:** voiceprints are biometric. Cascade-delete them with the person, list and delete them in Settings, never log vectors, and consider Keychain-backed encryption (anarlog stores the vectors in the keyring).

### CAP-08 · Recording-only mode and post-pass transcription
- **Read first:**
  - `Handy/src-tauri/src/engine_supervisor/supervisor.rs:50-107, 860-896` (batch deadlines and retry)
  - `TC/src/session.rs:171-270` (`run` and `run_batch`, partial transcripts on `OutputTruncated`)
  - `anarlog/crates/listener2-core/src/batch/{mod,diarize}.rs` (batch, then diarize, then progress heartbeats)
- **Live transcription off:**
  - Load no STT model and start no worker processes. Run only capture, encode and a cheap VAD (Earshot, pure Rust) if CAP-07 auto-stop needs it.
  - Verify the 15% CPU target with the per-process probe.
- **Stage `final_transcription`:**
  - Decode `mic.m4a` and `system.m4a` separately.
  - VAD-segment each into utterances of at most 30 s, or `capabilities.max_audio_ms`.
  - Call `BatchTranscriber::transcribe` per utterance with `TimestampKind::Word`; use `run_batch` for throughput.
  - Write segments with absolute times and `channel`, then let the `diarize` and `identify_speakers` stages run as usual.
- **High-accuracy re-pass:**
  - The same pipeline, optionally with a different offline model from the catalog (Parakeet TDT 0.6B v3 GGUF exists).
  - Replace live segments in one transaction, carrying speakers over by maximum word-time overlap (`assign_words`). Keep renamed speakers.
- **Progress and timeouts:**
  - Report progress per utterance, with a heartbeat at least every 5 s (anarlog).
  - The supervisor's run deadline scales with audio length (GPU: max(60 s, 10× audio)), so feed utterances, not the whole hour.
- **Errors:**
  - Handle `Aborted`, `OutputTruncated` and `OutputRepetition` by keeping the partial transcript and marking the segment `needs_review`, not by failing the job.
  - A worker crash retries once on CPU (Handy behaviour); the job resumes at the first utterance with no segments, so it is idempotent.
- **Settings:** "Live transcription: On/Off" and "High-accuracy re-pass" are typed settings (FND-06). The pipeline stage reads them, never the lane code.

---

## Appendix A: model artifacts

| Purpose | Artifact | Source (pinned) | Size (bytes) | sha256 |
|---|---|---|---|---|
| STT (default) | `parakeet-unified-en-0.6b-Q8_0.gguf` | `huggingface.co/handy-computer/parakeet-unified-en-0.6b-gguf/resolve/7e948f21b7bdbac698d3318db9d350f1096f3b6c/<file>` | 731,357,568 | `4b50b6dd862bf6e346929aaf4f5eaacec003bfa3f56462d6c874b41ef2f38795` |
| STT | `…-Q4_K_M.gguf` | same | 477,274,496 | `a8bf3de2b393bd14ead5a858c3748d5e3b07a20fdeabdd3b498fba4f463fa929` |
| STT | `…-Q5_K_M.gguf` | same | 540,795,264 | `f9def6f9b4e83ab7d006df3e1b676dfa1f973a3b6da232a9c99fcaa66bcd2836` |
| STT | `…-Q6_K.gguf` | same | 602,191,232 | `912c42ec3fd1afa31b7eda45684577feadf413249ca767fa912a89c4810e38d0` |
| STT | `…-F16.gguf` | same | 1,239,114,240 | `4a284b229bff9dc66aa00666fdcc2419c0f82a5adb5a36f7a48a4e0713c35492` |
| STT | `…-F32.gguf` | same | 2,473,323,520 | `81d3a8c7676324a4dae977dac0f5900dc1468755fdf3b5369261bb759f17fdf3` |
| Segmentation | `sherpa-onnx-pyannote-segmentation-3-0.tar.bz2` (model.onnx + int8) | `github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/` | 6,958,444 | compute at pin time |
| Segmentation (anarlog copy) | `segmentation.onnx` | `anarlog/crates/pyannote-local/src/data/` | 5,986,908 | compute at pin time |
| Embedding (recommended) | `wespeaker_en_voxceleb_resnet34_LM.onnx` (256-d) | `github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/` | 26,530,550 | compute at pin time |
| Embedding (anarlog copy) | `embedding.onnx` (WeSpeaker, 256-d, input `feats`) | `anarlog/crates/embedding/src/onnx/` | 26,543,975 | compute at pin time |
| Embedding (alt) | `3dspeaker_speech_campplus_sv_en_voxceleb_16k.onnx` (512-d; openwhispr's choice) | same release tag | 29,596,978 | compute at pin time |
| Embedding (alt) | `wespeaker_en_voxceleb_CAM++.onnx` | same | 29,292,684 | compute at pin time |
| Embedding (alt, larger) | `nemo_en_titanet_small.onnx` / `nemo_en_titanet_large.onnx` | same | 40,257,283 / 101,405,493 | compute at pin time |
| Avoid (licence) | `sherpa-onnx-reverb-diarization-v1/v2.tar.bz2` | segmentation tag | 10,918,585 / 254,075,989 | n/a |

The STT sizes and hashes come from `Handy/src-tauri/src/catalog/catalog.json:29-37`. The ONNX sizes come from the GitHub releases API (`/repos/k2-fsa/sherpa-onnx/releases/tags/{speaker-segmentation-models,speaker-recongition-models}`); "recongition" is the upstream spelling of the tag. GitHub publishes no digest for these assets.

Licences to add to `THIRD_PARTY_NOTICES.md`:
- Handy (MIT), transcribe.cpp (MIT), anarlog (MIT, non-enterprise paths only), openwhispr (MIT, if any logic is ported), sherpa-onnx (Apache-2.0, if used).
- Model weights: Parakeet Unified EN (CC-BY-4.0, per the catalog), pyannote segmentation-3.0 (MIT), WeSpeaker ResNet34-LM (CC-BY-4.0), 3D-Speaker CAM++ (Apache-2.0).

## Appendix B: other transcribe.cpp models relevant later
- `handy-computer/parakeet-tdt-0.6b-v3-gguf` (Q8_0, 739,508,576 B; token timestamps, offline). It can replace Deino's ONNX Parakeet TDT "Advanced" option on the same engine.
- `handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf` (streaming, 28 languages). This is the upgrade path if multilingual support is ever in scope.
- `handy-computer/multitalker-parakeet-streaming-0.6b-v1-gguf` plus its `bundle/` variant: Parakeet with an embedded Sortformer. It gives speaker-attributed segments, at most 4 speakers, and no embeddings.
- Standalone Sortformer `diar_streaming_sortformer_4spk-v2.1`: run-slot `RunExtension::Sortformer(SortformerStreamOptions { preset })` with presets `VeryHighLatency` (≈30 s lookahead, offline), `HighLatency` (≈10 s) and `LowLatency` (≈1 s). Handy's catalog excludes it on purpose (`catalog/mod.rs:235-243`).
