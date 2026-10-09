# CAP-01 spike: Parakeet Unified EN 0.6B streaming via transcribe-cpp 0.3.1

Machine: Apple M4 Max, 64 GB, macOS 26 (Darwin 25.6), Rust 1.92. Date: 2026-10-08.
Crate: `stt-spike` in this folder (`cargo build --release`). Models are in `../models/`;
`scripts/fetch_models.sh` re-downloads them from the pinned revision and verifies sha256.
Q8_0 `4b50b6dd…f38795` and Q5_K_M `f9def6f9…2836` both verified.

> **Measurement caveat: read this before quoting any wall-clock number.**
> From about 21:00 onward the machine was shared with parallel agent builds and benchmarks.
> The worst was another spike's diarization bench at 350-800 % CPU, with load average 15-155 on 16 cores.
> A Minecraft client also used about 35 % of the GPU for the whole session.
> Every timing below is labelled with the 1-minute load average it ran under (**L=**).
> Runs before about 20:20 have no recorded load, but their chunk compute matched quiet expectations (**L=low?**).
>
> - **Audio-clock latency** = `input_received_ms` at emission minus the token's `t1_ms`. It is deterministic: it depends only on the window, not on load. Trust these.
> - **Wall-clock latency** = audio-clock latency + chunk compute + queueing.
>   - Chunk compute for one stream: 34-46 ms p50 at L=low?.
>   - Under load: 150-500 ms p50, with p95 up to 1.1 s.

---

## 1. Headline answers

| Question | Answer |
|---|---|
| Works on Metal? | Yes. `model.backend()` = `"MTL0"`. Footprint per loaded model after warmup: 0.98 GB (Q8_0), 0.80 GB (Q5_K_M). |
| Load time | Warm: 175-320 ms. **First run on the machine (Metal library compile): 17.3 s.** Seen twice: the first-ever run, and again mid-session in a fresh process after the shared system shader cache was evicted. |
| Batch RTF | 0.004-0.007, i.e. 150-250x real time (L=low?). Under load, 0.02-0.03. |
| Default streaming latency | 1.52 s (p50) / 2.0 s (p95) from word end to text, on the audio clock. 1.95 s (p50) / 2.44 s (p95) from utterance onset to first text. This confirms the plan's "about 2.1 s". |
| Reach about 1.5 s? | **Yes, with window (L, C, R) = (5600, 560, 320) ms.**<br>Audio clock: token p50 0.56 s / p95 0.80 s; onset-to-first-text p50 0.85 s / p95 1.23 s.<br>WER is unchanged on the fixtures. GPU cost is 1.8x the default (RTF about 0.06). |
| Committed vs tentative | Parakeet commits every decoded token immediately. **`tentative` is always empty** under `Auto` and `StablePrefix`, and `stable_prefix_agreement_n` has no effect. `OnFinalize` makes everything tentative until finalize. The window is the only latency knob. |
| Timestamps | Token timestamps on both paths, in `Transcript::tokens` from `run()` and from `Stream::snapshot()`.<br>**`Transcript::words` is empty when streaming**, so group tokens into words yourself (`src/words.rs`).<br>Feeding continuous (not VAD-gated) audio keeps tokens on the wall-clock timeline: **no drift over 82 minutes** (±40 ms).<br>`t0` runs about 200-250 ms late against Deepgram word onsets. |
| Two streams | A second `stream()`, or a `run()`, on the same `Model` returns `Error::Busy`.<br>Two `Model` instances in **one process** stream concurrently without problems. Two processes also work.<br>About 1.55 GB (in-process) or about 2.0 GB (two processes) for two Q8_0 lanes.<br>No measurable latency penalty from the second stream; outside load dominates. |
| Q5_K_M vs Q8_0 | Q5_K_M uses about 180 MB less per model. WER is the same: 9.84 vs 9.67 % on english_3, 0.40 % on the synthetic conversation for both. Speed differences are within noise. |
| whisper-rs 0.13 + transcribe-cpp | **Links without error, then aborts at runtime.**<br>Only one copy of each `ggml_*` symbol survives the link, so transcribe-cpp ends up calling whisper.cpp's older ggml.<br>The abort: `GGML_ASSERT(key_id >= 0 && key_id < gguf_get_n_kv(ctx))`, SIGABRT. |

---

## 2. API calls that worked (transcribe-cpp 0.3.1)

```toml
transcribe-cpp = { version = "=0.3.1", default-features = false, features = ["metal", "serde"] }
```

```rust
// Before ANY Metal/ggml init (Handy lib.rs:626-638). Also `_exit` at shutdown to skip ggml-metal teardown.
std::env::set_var("GGML_METAL_NO_RESIDENCY", "1");
let devices = transcribe_cpp::devices();     // ~40-100 ms warm; compiles the Metal library when cold
let model = Model::load_with(path, &ModelOptions { backend: Backend::Metal, device: None })?;
assert_eq!(model.backend(), "MTL0");         // anything else = CPU fallback -> health warning
let caps = model.capabilities();
// Capabilities { native_sample_rate: 16000, languages: ["en"], max_timestamp_kind: Token,
//                supports_streaming: true, max_audio_ms: 0 /* no limit */, .. }
```

The streaming window menu is in the GGUF header (`src/gguf.rs`, `stt-spike info`). Each field is validated independently, so any C×R pair is accepted:

```
stt.parakeet.encoder.att_chunk_left_choices  = [70]               -> L = 5600 ms only
stt.parakeet.encoder.att_chunk_chunk_choices = [1, 2, 7, 13]      -> C = 80, 160, 560, 1040 ms
stt.parakeet.encoder.att_chunk_right_choices = [0,1,2,3,4,7,13]   -> R = 0, 80, 160, 240, 320, 560, 1040 ms
```

Streaming (`src/stream.rs`):

```rust
let run = RunOptions { timestamps: TimestampKind::Token, ..Default::default() };
let opts = StreamOptions {
    commit_policy: CommitPolicy::Auto,        // Parakeet: native commit; tentative stays empty
    stable_prefix_agreement_n: 0,             // ignored for Parakeet
    family: Some(StreamExtension::ParakeetBuffered(ParakeetBufferedStreamOptions {
        left_ms: Some(5600), chunk_ms: Some(560), right_ms: Some(320),
    })),
};
let mut session = model.session()?;
let mut stream = session.stream(&run, &opts)?;   // Err(Error::Busy) if this Model already streams/runs
for frame in frames {                            // 20/30/100 ms: all fine
    let up: StreamUpdate = stream.feed(&frame)?; // ~10 µs unless a chunk is due, then one encoder pass
    if up.committed_changed || up.tentative_changed {
        let text = stream.text();                // StreamText { full, committed, tentative }
        let snap = stream.snapshot();            // snap.tokens grows (append-only); snap.words is EMPTY
        // backlog: up.buffered_ms == up.input_received_ms - up.audio_committed_ms
    }
}
stream.finalize()?;                              // 40-120 ms (L=low?)
let fin = stream.snapshot();                     // fin.tokens: Vec<Token { t0_ms, t1_ms, p, text, .. }>
let words = words::words_from_tokens(&fin.tokens, stream_start_ms); // src/words.rs (unit-tested)
```

Batch:

```rust
let tr = session.run(&pcm, &RunOptions { timestamps: TimestampKind::Token, ..Default::default() })?;
// tr.words AND tr.tokens are both populated; tr.timings = { load_ms, mel_ms, encode_ms, decode_ms }
```

`stt-spike busy`, verbatim:

```
second stream on same Model: Error::Busy("a stream is already active on this model")
run() on same Model while streaming: Error::Busy("a stream is active on this model; finish or drop it before run()")
stream on a second Model instance: OK (load 371 ms)
footprint with two models loaded: 1535 MB
```

The busy flag is a per-`Model` `Mutex<bool>` (`compute_lock` in `TC/src/model.rs`). A second `Model::load_with` of the same file gets its own lock, its own weights and its own Metal buffers.

Sample timestamps:

```
batch tokens:  960-1040 " G" p=0.998 | 960-1040 "ood" | 1040-1120 " mor" | 1040-1120 "ning" p=0.938
batch words:   960-1040 "Good" | 1040-1120 "morning" | 1360-2000 "Daniel,"
stream tokens around an 8 s silence gap (continuous audio, so these are true meeting times):
  47040 ' m' 'ach' 'ines'   <- "machines" (speech ends at 47548)
  56640 '.'                 <- the period arrives 9 s late, with the next speech
  56720 ' A' 'fter' ...     <- next utterance (speech starts 55548; "Okay." was dropped)
```

---

## 3. Load and warmup (`stt-spike load`; each process is one sample)

| Metric | Q8_0, warm (3 runs, L=low?) | Q8_0, warm (L≈18) | Q5_K_M, warm (L≈35) | First run on the machine |
|---|---|---|---|---|
| `devices()` | 39-43 ms | 99 ms | 53 ms | included in load |
| `Model::load_with` | 175-198 ms | 318 ms | 188 ms | **17,312 ms** (first ever), **17,636 ms** (fresh process, mid-session) |
| Second load in the same process | 185-202 ms | 277 ms | 171 ms | — |
| First stream (1 s of zeros) | 54-59 ms | 72 ms | 165 ms | 442 ms (per-kernel pipeline compile) |
| `run()` on 1 s, first / second | 26-29 / 25-29 ms | 36 / 51 ms | 65 / 37 ms | 117 / 25 ms |
| Footprint after load / after warmup | 800 / 977 MB | 801 / 978 MB | 621 / 800 MB | |

- **The 17 s is the ggml Metal library compile.**
  - transcribe-cpp-sys builds with `GGML_METAL_EMBED_LIBRARY=ON`, so the `.metal` source is compiled at runtime.
  - The OS caches the result in `$(getconf DARWIN_USER_CACHE_DIR)/com.apple.metal/`. That cache is shared and can be evicted, which happened once during this session.
  - Expect the 17 s after OS or app updates and after cache eviction, not only on first install.
  - Handy's 60 s hello timeout and background pre-warm are required. (I did not delete the shared cache to force a reproducible cold run.)
- The warmup stream pays a smaller per-kernel pipeline compile of about 0.4 s.

## 4. Batch transcription (Metal, best of 3)

| Audio | Duration | Best run | RTF | encode / decode ms | WER | Load |
|---|---:|---:|---:|---|---:|---|
| Synthetic 2-voice conversation (`say`, Samantha/Daniel) | 103 s | 515 ms | 0.0050 | 403 / 97 | 0.40 % | L=low? |
| anarlog `english_3` (real meeting; reference = Deepgram words) | 245 s | 1,654 ms | 0.0067 | 1364 / 257 | 9.67 % | L=low? |
| LibriVox Gettysburg Address (public domain, fetched) | 158 s | 837 ms | 0.0053 | 760 / 55 | **stopped after 22 words** | L=low? |
| english_3, Q5_K_M | 245 s | 5,287 ms | 0.022 | 1972 / 3269 | 9.84 % | L≈35 |
| english_3, Q8_0 | 245 s | 6,900 ms | 0.028 | 2242 / 4598 | 9.67 % | L≈18 |

About 5-7 points of the english_3 WER come from noise in the reference itself and from normalisation (Deepgram's lower-case transcript, no ITN). Use it only to compare runs against each other.

**Surprise: batch decoding can stop early.**
- On the LibriVox file, `run()` emitted only the 22-word preamble ("…please visit LibriVox.") and then nothing for the remaining 2.3 minutes of speech.
- Whether it happens depends on where the clip is cut:
  - Stops early: 0-30 s, 0-60 s, 0-158 s.
  - Fine: 30-60 s, 0-120 s.
- **Streaming the same file works with every window** (about 305 words).
- For CAP-01 import, retranscribe and CAP-08:
  - Run batch over VAD-bounded chunks of 30 s or less.
  - Add a words-per-speech-minute sanity check, with a stream re-run as the fallback.

**The RNN-T decoder runs on the CPU.** Under CPU contention, batch decode went from 257 ms to 3.3-4.6 s while the GPU encoder barely moved.

## 5. Streaming

### 5.1 Window sweep: accuracy and algorithmic latency

`scripts/sweep_windows.sh` runs `--fast` with 100 ms frames and L = 5600, measured on the audio clock. Latency figures use the synthetic conversation (17 utterances):
- *token lat*: when the token appears, minus the token's end.
- *onset lat*: when an utterance's first token appears, minus the true speech onset.

Chunk compute is per encoder pass for one stream. Rows marked * ran after the machine became loaded (about 20:30 onward); the other rows are L=low?.

| C ms | R ms | conv WER | e3 WER | LibriVox words | token lat p50/p95 | onset lat p50/p95 | chunk compute p50 | stream RTF |
|---:|---:|---:|---:|---:|---|---|---:|---:|
| 1040 | 1040 (default) | 0.40% | 12.13% | 305 | 1560 / 2000 | 1979 / 2516 | 36.6 ms | 0.035 |
| 1040 | 560 | 0.81% | 11.42% | 306 | 1080 / 1520 | 1485 / 2016 | 34.6 ms | 0.034 |
| 1040 | 0 | 1.62% | 12.48% | 306 | 540 / 980 | 985 / 1652 | 34.0 ms | 0.033 |
| 560 | 1040 | 0.40% | 10.37% | 305 | 1320 / 1580 | 1700 / 2006 | 34.4 ms | 0.063 |
| 560 | 560 | 0.40% | 11.07% | 306 | 840 / 1080 | 1169 / 1506 | 35.4 ms | 0.064 |
| **560** | **320** | 0.81% | **9.67%** | 305 | **600 / 860** | **932 / 1285** | 33.6 ms | 0.060 |
| 560 | 160 | 0.81% | 10.19% | 305 | 440 / 680 | 769 / 1106 | 38.0 ms | 0.079 |
| 560 | 0 | 2.02% | 13.36% | 305 | 280 / 540 | 569 / 1006 | 44.8 ms | 0.104 |
| 160 | 560 | 0.81% | 9.67% | 306 | 640 / 720 | 979 / 1200 | 35.0 ms | 0.219 |
| 160 | 320 | 0.81% | 10.37% | 303 | 400 / 480 | 758 / 1000 | 36.3 ms | 0.251 |
| 160 | 160 | 0.81% | 12.65% | 303 | 240 / 320 | 569 / 800 | 64.0 ms* | 0.562* |
| 160 | 0 | 25.10% | 20.56% | 298 | 80 / 160 | 506 / 618 | 55.8 ms* | 0.450* |
| 80 | 80 | 6.88% | 13.88% | 303 | 120 / 160 | 458 / 700 | 66.3 ms* | 0.954* |
| 80 | 0 | 34.01% | — | — | 40 / 80 | 476 / 700 | 74.9 ms* | 0.969* |

How to read the table:
- **Algorithmic latency ≈ C/2 + R at p50, and C + R at the maximum.**
- **Every chunk re-encodes the whole [L|C|R] window**, about 6-7 s of audio, because the buffered path keeps no encoder cache. Compute per chunk is therefore roughly flat (about 35 ms), and the GPU duty cycle is about 35 ms / C:
  - C = 1040: 3.5 %
  - C = 560: 6 %
  - C = 160: 22 %
  - C = 80: about 50 % or more, which is not viable for two lanes.
- R = 0 costs accuracy, badly so when C ≤ 160. With C = 560, any R ≥ 160 costs nothing on these fixtures.
- **Recommendation:** default to **(5600, 560, 320)**. Offer (5600, 1040, 1040) as "max accuracy / low power".

### 5.2 Real-time paced runs (continuous, not VAD-gated; mic-like producer thread)

Full tables are generated by `scripts/summarize_rt.py results/**/*.json`.

| Run | C/R | Frame | Load | first text | token lat p50/p95/max (wall) | onset lat p50/p95 (wall) | chunk compute p50/p95/max | max `buffered_ms` | RTF | peak footprint |
|---|---|---:|---|---:|---|---|---|---:|---:|---:|
| default | 1040/1040 | 20 | L=low? | 2129 | 1569 / 2046 / 2088 | 1994 / 2509 | 46 / 73 / 102 | 2060 | 0.048 | 998 MB |
| default | 1040/1040 | 20 | L≈82 | 2324 | 1708 / 2286 / 2532 | 2102 / 2605 | 156 / 478 / 606 | 2060 | 0.187 | 1001 MB |
| default | 1040/1040 | 30 | L≈64 | 2560 | 1713 / 2207 / 2454 | 2103 / 2557 | 148 / 397 / 509 | 2070 | 0.175 | 999 MB |
| default | 1040/1040 | 100 | L≈117 | 2240 | 1906 / 2603 / 3409 | 2159 / 3150 | 247 / 827 / 1328 | 2060 | 0.317 | 1001 MB |
| default, Q8_0 | 1040/1040 | 20 | L≈18 | 2133 | 1596 / 2079 / 2526 | 2006 / 2506 | 67 / 237 / 525 | 2060 | 0.095 | 998 MB |
| 560/320 | 560/320 | 20 | L≈91 | 1614 | 830 / 1188 / 1718 | 1071 / 1451 | 216 / 463 / 910 | 860 | 0.429 | 996 MB |
| 560/560 | 560/560 | 20 | L≈102 | 2454 | 934 / 1414 / 2188 | 1236 / 2214 | 69 / 409 / 1094 | 1100 | 0.245 | 999 MB |
| 160/560 | 160/560 | 20 | L≈99 | 1710 | 672 / 708 / 752 | 991 / 1210 | 34 / 69 / 114 | 700 | 0.242 | 997 MB |
| CPU backend, `--fast` | 1040/1040 | 100 | L≈21 | — | — | — | 1014 / 1737 / 3143 | 2060 | **1.05** | — |

**Load-gated re-run (`scripts/run_key.sh`, results in `results/rt_key/`).** The rule was: wait up to 180 s for load < 20, otherwise run anyway and record the load. The load never dropped below 80, so every row below ran heavily loaded (L≈80-184).

| Run | C/R | Load (start→end) | first text | token lat p50/p95/max (wall) | onset lat p50/p95 (wall) | chunk compute p50/p95/max | max queue lag | RTF |
|---|---|---|---:|---|---|---|---:|---:|
| default, 1 stream | 1040/1040 | 120→183 | 2205 | 1807 / 2466 / 3144 | 2262 / 2545 | 214 / 608 / 1140 | 1124 | 0.277 |
| 560/320, 1 stream | 560/320 | 157→79 | 1499 | 668 / 1027 / 1647 | 936 / 1336 | 99 / 294 / 842 | 827 | 0.230 |
| 560/320, 2 processes: mic | 560/320 | 182→138 | 1514 | 803 / 1388 / 1746 | 1014 / 1489 | 194 / 637 / 816 | 1086 | 0.436 |
| 560/320, 2 processes: system | 560/320 | 182→138 | 1568 | 1044 / 2278 / 3176 | — | 209 / 785 / 1225 | 2498 | 0.522 |
| batch english_3 (best of 3) | — | 181→184 | — | — | — | encode 1818 ms, **decode 11,964 ms** | — | 0.051 |

At a load of 140-180, 560/320 still met the 1.5 s onset target on p95: 1.34 s with one stream and 1.49 s with two. The system lane's tail did not; its token p95 was 2.3 s. Batch decode, which runs on the CPU, was 47x slower than when quiet (11,964 ms vs 257 ms), while the GPU encoder slowed only 1.3x. This is the clearest evidence that CPU starvation, not the GPU, drives degradation. A two-stream comparison on a quiet machine is still outstanding; re-run `scripts/run_key.sh` when `uptime` load is below 20.

Findings from these runs:
- **Frame size (20/30/100 ms) does not matter to the engine.**
  - `feed` buffers until a chunk is complete and costs about 10 µs otherwise. Only the frame that completes a chunk pays for the encoder.
  - 100 ms frames add up to one frame of latency.
  - Feed 20 ms frames in-process, or 160 ms frames over IPC.
- **Backlog.**
  - `buffered_ms` is exactly `input_received_ms − audio_committed_ms`. It follows a sawtooth between R and C+R: 1040↔2060 for the default window, 320↔860 for 560/320. That is not a growing backlog.
  - A real backlog shows up only as *our* queue lag, i.e. frames waiting while `feed` computes. Under load it reached 1-13 s (13 s at 560/320 with L>100) and then recovered.
  - CAP-01 should alarm on queue lag > 2×C, not on `buffered_ms`.
- **The CPU fallback cannot keep up** at the default window (RTF 1.05, L≈21). A worker that has fallen back to CPU must switch to batch-after-utterance.
- **Tail latency is driven by CPU contention, not by the GPU.** The decoder and mel run on the host. Run worker threads at `QOS_CLASS_USER_INITIATED` or higher (not tested here).

### 5.3 Commit policies (default window, 100 ms frames)

| Policy | committed changes | tentative changes | tentative ever non-empty | Behaviour |
|---|---:|---:|---|---|
| `Auto` | 87 | 0 | no | Every decoded token is committed immediately (`FamilyNativeCommit`, `TCS/src/transcribe.cpp:1444`). |
| `StablePrefix`, `agreement_n=2` | 87 | 0 | no | Identical to `Auto`; `agreement_n` is never consulted for Parakeet. |
| `OnFinalize` | 0 | 87 | yes (up to 1366 chars) | The same text arrives at the same time, but all of it stays tentative until `finalize()`. |

There is no tentative region to tune: buffered Parakeet never rewrites a decoded token (`model.cpp:3005-3012`). Display `committed` text. Keep the committed/tentative split in the protocol for other model families.

### 5.4 Timestamps on continuous audio (`scripts/align.py`; words rebuilt from tokens)

| Check | Result |
|---|---|
| english_3 vs Deepgram word starts, batch | 519/569 words matched. `t0` hyp−ref: mean **+226 ms**, median abs 215 ms, p95 abs 410 ms. |
| Same check, streaming at 1040/1040 · 560/320 · 160/560 | Mean +238 / +239 / +243 ms; p95 abs 410 ms. The window does not affect timestamps. |
| Drift within 245 s (first vs last quarter) | +284 → +219 ms. No growth. |
| **Drift over 82 minutes** (english_3 looped 20×, one stream, L≈43) | Tokens of loop *k* vs loop 0, shifted by *k*·245.098 s: median offsets **+22, −36, +29, −22, +6, +13 ms** for loops 1, 2, 5, 10, 15, 19. **No drift.** |
| Synthetic conversation: first word vs speech onset | +350 to +460 ms (`say` onsets are soft). No word lands in a silence gap, including right after the 8 s gap. |

- With ungated audio, `meeting_ms = stream_start_ms + t0_ms` holds exactly.
- **Bias.** `t0` is the frame where the RNN-T emits the token, about 200-250 ms after the acoustic onset. Optionally calibrate it with −200 ms.
- **`t1` is not a word end.** It is `t0` + one 80 ms frame. Use the last non-punctuation token's `t1`, or the next word's `t0`.
- **Late punctuation.** The sentence-final "." before a pause is emitted when the next speech starts, and stamped with that later time (9 s later in the sample above). `words_from_tokens` attaches the punctuation text without extending the word's end time. Segment on VAD silence, not on ".".
- **A word dropped after long silence.** "Okay." right after the 8 s gap was missing in every streaming window but present in batch. One example only, but a hint.

### 5.5 One 82-minute stream (`--fast`, default window, L≈43)

| Metric | Value |
|---|---|
| Audio, chunks, RTF | 4902 s, 4712 chunks, RTF 0.125 |
| `feed` p50 | 0.14 ms (vs 0.01 ms on short streams; the text is rebuilt in O(tokens) per chunk) |
| `snapshot()` p50 | 0.65 ms (vs 0.01-0.02 ms; it copies every token) |
| Footprint | Peak 2.15 GB. This includes 2 × 314 MB of the spike's own looped input. Growth on the engine side is ≤ about 200 MB (noisy). |

Source inspection explains the growth:
- The buffered path never trims `stream_pcm_buffer` (`model.cpp:2967`; it is only cleared at begin and finalize). That is about 230 MB per hour of f32 audio.
- `rebuild_streaming_result_text` re-detokenises everything on every chunk (`model.cpp:2241`).

A stream as long as the whole meeting works and does not drift, but its cost grows linearly. **Roll streams** at VAD silences, at least every 2-5 minutes, or make them utterance-scoped.

## 6. Two concurrent streams (mic + system lanes)

Setup: real-time paced, 20 ms frames. Mic = synthetic conversation; system = english_3.

| Setup | Load | Model loads | Footprint | mic token lat p50/p95 (wall) | system token lat p50/p95 | chunk compute p50/p95 (mic · system) | GPU avg (system-wide; ~35 % baseline from another app) |
|---|---|---|---|---|---|---|---|
| 1 process, 2 `Model`s, default | L≈68 | 209 + 168 ms | 1554 MB after loads, **1811 MB peak** | 1806 / 2481 | 1793 / 2489 | 221 / 770 · 199 / 640 | 52 % |
| 1 process, 2 `Model`s, 560/320 | L≈78 | 17636 (cold Metal) + 303 ms | 1574 → **1826 MB peak** | 626 / 882 | 636 / 890 | 70 / 121 · 72 / 121 | 81 % |
| 2 processes, default | L≈155 | ≈ single | **1021 + 1043 MB** | 1627 / 2068 | 1584 / 2076 | 65 / 200 · 59 / 214 | 41-43 % |
| 2 processes, 560/320 | L≈87 | ≈ single | **997 + 1049 MB** | 610 / 854 | 632 / 877 | 46 / 83 · 51 / 110 | 51-71 % |
| Reference: single stream, default | L=low? / L≈18 | 175-318 ms | 998 MB | 1569 / 2046 · 1596 / 2079 | — | 46 / 73 · 67 / 237 | 41 % |

- **One `Model` cannot serve two streams.** It returns `Error::Busy` (§2).
- **Two `Model` instances in one process run truly concurrently.** The per-model lock is the only serialisation, and ggml-metal raised no asserts over 2 × 103 s.
- **No measurable latency degradation from the second stream.** At C = 560 the GPU is about 95 % idle per lane; variance from outside load is far larger than any two-stream effect.
- **Memory.**
  - Two models in one process: about 1.55 GB, peaking at 1.8 GB.
  - Two processes: about 2.0 GB. That is about 0.25 GB extra for the duplicated runtime and Metal library.
  - Each active stream adds about 180 MB over the bare model.
- **CPU.** 0.06-0.25 cores per stream on average.

## 7. Q5_K_M vs Q8_0

| | Q5_K_M | Q8_0 |
|---|---:|---:|
| File size | 541 MB | 731 MB |
| Footprint after load / after warmup | 621 / 800 MB | 800 / 977 MB |
| Warm load | 188 ms (L≈35) | 175-318 ms |
| Streaming, default window, 20 ms frames: token lat p50/p95 | 1656 / 2173 ms (L≈35) | 1596 / 2079 ms (L≈18) |
| Chunk compute p50/p95 | 75 / 461 ms (L≈35) | 67 / 237 ms (L≈18) |
| WER, conversation / english_3 (batch) | 0.40 % / 9.84 % | 0.40 % / 9.67 % |

Q5_K_M saves about 180 MB per lane with no measurable accuracy loss. A speed comparison would need a quiet machine; on these runs the two are within noise.

## 8. whisper-rs 0.13.2 + transcribe-cpp 0.3.1 in one binary (`link-test/`)

The test crate uses Deino's macOS features: `whisper-rs = { 0.13.2, features = ["raw-api", "metal", "coreml"] }`.

1. **`cargo build` succeeds with no duplicate-symbol errors.** Both sys crates link a `libggml.a`, yet the final binary has exactly one definition of each `_ggml_*` symbol (243 exported). The linker silently kept one copy.
2. **At runtime:**
   - `transcribe_cpp::devices()` works.
   - whisper-rs transcribes `jfk.wav` correctly with `ggml-tiny.en.bin` (sha256 checked against HF).
   - `transcribe_cpp::Model::load` then aborts: `whisper-rs-sys/.../whisper.cpp/ggml/src/ggml.c:22579: GGML_ASSERT(key_id >= 0 && key_id < gguf_get_n_kv(ctx)) failed`, exit 134. transcribe.cpp's GGUF reader is running against whisper.cpp's older ggml. Log: `results/link_test.log`.
3. **Conclusion: remove `whisper-rs` before adding transcribe-cpp**, and run Whisper GGUFs through transcribe-cpp as Handy does.
   - The `--stt-worker` re-exec of the same binary **does not** avoid the clash: the worker links exactly what the app links.
   - The only alternative is a separate sidecar binary.

---

## 9. Recommendations for CAP-01

**Worker-process layout**
- One supervised worker per lane (`deino --stt-worker`, using Handy's supervisor), with one `Model` per worker. Two workers cost about 2.0 GB.
- A single worker holding two `Model`s also works and saves about 0.25 GB. But it puts both lanes in one crash domain, so keep it only as a memory fallback.
- Worker boot sequence:
  1. Set `GGML_METAL_NO_RESIDENCY=1`.
  2. Call `devices()`.
  3. `load_with(Backend::Auto)`, then assert `backend() == "MTL0"`.
  4. Warm up by streaming 2 s of zeros, then finalizing.
  5. Report `load_ms` and `warmup_ms`.
- Hello/load timeout of 60 s or more, because the 17 s Metal compile recurs whenever the shader cache is evicted. Pre-warm both lane workers in parallel; a warm load takes about 0.2 s.
- Run at elevated QoS. Log Handy-style `StreamPerf`, plus queue lag, plus the load average or thermal state at the time, so latency reports can be interpreted.

**Stream options**
- Default to `CommitPolicy::Auto` + `ParakeetBuffered { 5600, 560, 320 }`. Read the menu from the GGUF at load and validate the choice against it.
- Expose two settings:
  - "Responsive": 560/320, the default.
  - "Accurate / low power": 1040/1040, also used automatically on CPU fallback or battery saver.
- Never offer C ≤ 160 for two lanes, and never R = 0.
- Feed **all** audio, silence included, so the stream clock stays equal to the capture clock.
- Roll the stream at VAD silence every 2-5 minutes, recording each stream's start sample. This bounds the PCM buffer, the text-rebuild cost and the snapshot cost.
- Call `snapshot()` only when `committed_changed`, and send only the new tokens over IPC. Tokens are append-only for Parakeet.

**Timestamps**
- Persist words built with `words_from_tokens` (`src/words.rs`), because `Transcript::words` is empty when streaming.
  - Word start = the first token's `t0`.
  - Word end = the last non-punctuation token's `t1`.
- Optionally shift onsets by −200 ms.
- Batch and stream timestamps agree to within about 10-20 ms, so the batch path's timestamps can be stored the same way.

**Memory defaults by RAM (two lanes)**

| RAM | Default | Footprint |
|---|---|---|
| ≥ 32 GB | Q8_0 × 2 workers | about 2.0-2.1 GB |
| 16-24 GB | Q5_K_M × 2 workers (Q8_0 also OK) | about 1.65 GB |
| 8 GB | Q5_K_M × 2 at 1040/1040, or one time-multiplexed model (Q4_K_M untested) | about 1.65 GB / 0.8 GB |

**How realistic is the latency target?**
- **1.5 s is achievable with 560/320.**
  - Onset to first text: p50 0.85 s / p95 1.23 s on the audio clock.
  - Add about 50-100 ms of compute on an idle M-series GPU, plus IPC and UI time.
  - The wall-clock results that fit within target were measured at L≈78-87 with two lanes: 0.61-0.64 s p50 / 0.85-0.89 s p95 token lag, and 0.90 s p50 / 1.28-1.35 s p95 onset.
- **The default window cannot meet 1.5 s.** Onset p50 is 2.0 s and p95 is 2.5 s.
- Heavy CPU contention adds seconds to the tail whatever the window. Run the acceptance test on an idle machine and log p50/p95.
- Base M1/M2 GPUs were not tested. Expect encoder passes 3-5x slower; that is still fine at C = 560 for two lanes.

## 10. Surprises

1. **Tentative text is always empty** for Parakeet, which commits natively; tuning the commit policy does nothing.
2. **Streaming snapshots carry tokens but no words.**
3. **Batch `run()` can stop emitting text** partway through a file (LibriVox: 2.3 of 2.6 minutes lost), depending on the clip boundaries. Streaming the same file is fine.
4. **whisper-rs and transcribe-cpp link silently, then crash at runtime**, because only one ggml survives the link.
5. **The 17 s Metal compile happened twice in one session.** The shared system shader cache does get evicted.
6. **The model licence doesn't match the docs.**
   - The GGUF metadata says `general.license = "other"`, `license.name = "nvidia-open-model-license"`.
   - Handy's `catalog.json` and Appendix A of audio-stt.md say `cc-by-4.0`.
   - Check the model card and `THIRD_PARTY_NOTICES.md`.
7. **Sentence-final punctuation is held back until the next speech**, which can be seconds later.
8. **The buffered stream never trims its PCM buffer** (about 230 MB per hour), and it rebuilds the text in O(n) per chunk.
9. **The decoder runs on the CPU**, so CPU contention, not the GPU, dominates tail latency.

---

## Files

- `src/`:
  - `main.rs`: CLI (`info`, `load`, `batch`, `stream`, `busy`, `dual`).
  - `stream.rs`: paced streaming bench and metrics.
  - `engine.rs`: load and `StreamOptions` builders.
  - `words.rs`: token-to-word grouping, with a test. **Port this.**
  - `gguf.rs`: window menu reader. Port Handy's `gguf_meta.rs` instead.
  - `metrics.rs`: footprint, CPU and GPU sampling.
  - `audio.rs`, `args.rs`.
- `scripts/`:
  - `fetch_models.sh`: pinned revision + sha256.
  - `make_conversation.py`, `conversation.txt`.
  - `sweep_windows.sh`, `run_realtime.sh`, `run_key.sh`: the load-gated reruns.
  - `wer.py`, `align.py`, `summarize_sweep.py`, `summarize_rt.py`.
- `link-test/`: the whisper-rs + transcribe-cpp link and runtime test.
- `audio/`:
  - `conversation_16k.wav` + `conversation_truth.json`.
  - `librivox_gettysburg_16k.wav`, from archive.org `gettysburg_johng_librivox` (public domain).
  - english_3 is read in place from `deino-references/anarlog`.
- `results/`:
  - `sweep/`
  - `rt/` and `rt_loaded/` (loaded runs; the per-run load log is in `rt/load.log`)
  - `rt_key/` (load-gated reruns)
  - `stream_rt_default_f20.json` (L=low?)
  - `batch_*.json`
  - `link_test.log`

To reproduce:

```sh
scripts/fetch_models.sh && cargo build --release
./target/release/stt-spike stream --model ../models/parakeet-unified-en-0.6b-Q8_0.gguf \
  --wav audio/conversation_16k.wav --truth audio/conversation_truth.json --frame-ms 20 --window 5600,560,320
```
