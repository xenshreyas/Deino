# sherpa-onnx diarization and voiceprints (CAP-03, CAP-04)

`$R` = `~/Documents/GitHub/deino-references/sherpa-onnx` (Apache-2.0), version 1.13.8.

## Crate
- Use the **official `sherpa-onnx` crate 1.13.8** (`$R/sherpa-onnx/rust/sherpa-onnx`), not the third-party `sherpa-rs`.
- **Linking:** static by default, with the CPU provider. CoreML is compiled out when onnxruntime is linked statically, so stay on CPU unless a benchmark proves otherwise.
- **Build:** `build.rs` downloads prebuilt libraries from GitHub releases. Pin `SHERPA_ONNX_ARCHIVE_DIR` / `SHERPA_ONNX_LIB_DIR` for reproducible CI (`sherpa-onnx-sys/build.rs`).
- **No progress callback in Rust.** Add one `extern "C"` declaration for `SherpaOnnxOfflineSpeakerDiarizationProcessWithCallback` (`c-api.h:4073-4119`), which fires once per embedding.
- **Version conflict risk:** Deino already uses the `ort` crate (ONNX Runtime) for Parakeet TDT and Silero, and sherpa links its own onnxruntime statically. Check that both link together, or drop `ort` once CAP-01 removes the ONNX Parakeet default.

## Rust API
- **Diarization** (`src/offline_speaker_diarization.rs`):
  - `OfflineSpeakerDiarizationConfig { segmentation: { pyannote: {model, window_shift_ratio=0.1}, num_threads, provider }, embedding: SpeakerEmbeddingExtractorConfig, clustering: FastClusteringConfig { num_clusters=-1, threshold=0.5, compute_confidence }, min_duration_on=0.3, min_duration_off=0.5 }`
  - `OfflineSpeakerDiarization::create(&cfg)`, `.sample_rate()` (16k), `.process(&[f32]) -> result.sort_by_start_time() -> Vec<{start,end,speaker,confidence}>`. It is `Send + Sync`.
- **Embeddings** (`src/speaker_embedding.rs`):
  - `SpeakerEmbeddingExtractor::create/dim/create_stream/is_ready/compute -> Vec<f32>`. The stream: `accept_waveform(sr, &[f32])` then `input_finished()`. It resamples internally.
  - `SpeakerEmbeddingManager` is in-memory only: `add` L2-normalises, `add_list` averages into a centroid. `search`, `get_best_matches(emb, threshold, n)` and `verify` use **cosine similarity**.
- **VAD** (`src/vad.rs`): Silero. Defaults are threshold 0.5, min_silence 0.5, min_speech 0.25, window 512, max_speech 20.

## Pipeline facts
- **Segmentation:** pyannote-3.0 with a 10 s window and 1 s shift at ratio 0.1. At most 3 local speakers per chunk and 2 per frame. Overlapped frames are excluded from embeddings.
- **Clustering:** agglomerative, complete linkage, on cosine **distance** (1 − cos). A larger threshold means fewer speakers. Repo CI uses 0.90 with eres2net.
- **Output** can contain overlapping segments (up to 2 speakers per frame). Audio of 10 s or less skips clustering.
- **Input** must be 16 kHz mono f32. Diarization itself does not resample.
- **No online diarization** exists. For live use: VAD segments → embedding → running-centroid clustering in Rust.
- **Scaling risk:** with ratio 0.1 there are about 10 embeddings per active speaker per second.
  - A 1 h meeting with 2 speakers gives about 7.2k embeddings, and the distance matrix is about 200 MB.
  - A 2 h meeting with 3 speakers gives about 21.6k embeddings, about 1.9 GB.
  - **Use `window_shift_ratio` 0.25–0.5 for meetings over 30 minutes, or diarize in long overlapping blocks.** Benchmark it.
- **Benchmarking:** there is no DER tooling and there are no published numbers. Test wavs (16k): `1-two-speakers-en.wav`, `2-two-speakers-en.wav`, `3-two-speakers-en.wav`, `0-four-speakers-zh.wav` from the `speaker-segmentation-models` release.

## Models
Release base URL: `https://github.com/k2-fsa/sherpa-onnx/releases/download/` (`speaker-segmentation-models/`, `speaker-recongition-models/` — the misspelling is in the real URL — and `asr-models/silero_vad.onnx`).
- **Segmentation:** `sherpa-onnx-pyannote-segmentation-3-0.tar.bz2` (`model.onnx`, `model.int8.onnx`).
- **Embedding:** freeze ONE model and store its id with every voiceprint, because voiceprints from different models can't be compared.
  - Candidates for English: `3dspeaker_speech_campplus_sv_en_voxceleb_16k.onnx`, `wespeaker_en_voxceleb_resnet34_LM.onnx`, `nemo_en_titanet_small.onnx`.
  - Repo default: `3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx`.
  - Pick the model by measured DER on English fixtures.
- **Sizes and sha256:** not recorded in the repo. Fetch and pin them at implementation time.

## Recommended Deino flow
1. **After the meeting:** resample the system track (and the mic track if in-person) to 16 kHz.
   - `window_shift_ratio`: 0.1 for meetings of 30 minutes or less, 0.25 for longer ones.
   - `num_threads`: 4.
   - `num_clusters`: equal to the external attendee count if the calendar is reliable; otherwise use a threshold tuned on fixtures.
   - `compute_confidence = true`.
2. **Per cluster:** embed the longest clean segments (each at least 1.5 s, 10–30 s in total), then average the L2-normalised vectors into the cluster centroid.
3. **Cross-meeting matching:** persist the voiceprints as BLOBs with the model id.
   - Top similarity of 0.75 or more: auto-assign. Between 0.6 and 0.75: suggest. Calibrate per model.
   - Update the centroid when the user confirms.
4. **Live (optional):**
   - VAD segments of at least 1 s give embeddings, matched against the running centroids (join if similarity ≥ 0.55, otherwise a new provisional speaker).
   - The offline pass replaces the live labels afterwards.
