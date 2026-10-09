# 0019. Diarize locally with anarlog's pyannote port and WeSpeaker on Core ML

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** CAP-03

## Context

CAP-03 needs "who spoke when" for finished meetings, entirely on the Mac,
with per-speaker voice centroids for CAP-04's voiceprints. The plan listed
two engines (anarlog's `pyannote-local` or sherpa-onnx). The diarization
spike (`docs/references/spike-diarization.md`) measured both on synthetic
3–5 speaker meetings, including a 57-minute one, and decided the design:

- anarlog's engine with WeSpeaker ResNet34-LM embeddings: 8–12 % DER with a
  known speaker count, the count always right; sherpa-onnx lost speakers to
  outlier clusters (25–50 %) and links its own onnxruntime over `ort`'s;
- the embedding model on the `ort` Core ML execution provider (NeuralNetwork,
  CPU + Neural Engine), bit-identical to the CPU at about a tenth of the CPU
  time; segmentation on the CPU;
- calendar speaker bounds always passed (without them the 0.7045 threshold
  under-counts TTS voices: 25 % DER).

anarlog commits its models as blobs with no download source, embeds them with
`include_bytes!`, and pulls `knf-rs` from git.

## Decision

- **A new crate, `crates/deino-diarize`** (heavy native dependencies: `ort`,
  the C++ filterbank), ported from anarlog's `pyannote-local`, `embedding`,
  `onnx` and `voiceprint` (MIT) and split to §3A: `segmentation`,
  `embedding` (+ `features`), `clustering` (+ `linkage`), `pipeline`
  (`scan`, `labels`, `known`, `reconstruct`), `policy`, `prune`, `online`,
  `voiceprint`, `metrics`, `models`, `track`. No `tauri`, no `deino-net`.
- **Batching.** Segmentation runs 16 windows per `ort` call. Each window's
  filterbank is computed once and shared by its local speakers; embeddings
  of equal frame count run as one batch (statistics pooling makes padding
  inexact, so unequal lengths cannot share a call). Both are tested to give
  the same values as single runs.
- **Models, downloaded on demand** into `<app_data_dir>/models/diarization/`
  through `deino-core`'s verified, resumable downloader and its `ByteSource`
  port (the same path as the STT models), pinned by URL revision, byte size
  and SHA-256:
  - segmentation: Hugging Face `onnx-community/pyannote-segmentation-3.0`
    `onnx/model.onnx` @ `733a93b` (byte-identical to anarlog's, `057ee564…`);
  - embedding: Hugging Face `Wespeaker/wespeaker-voxceleb-resnet34-LM`
    `voxceleb_resnet34_LM.onnx` @ `f0c48c2` (`7bb2f06e…`), the authors' own
    export. Its embeddings equal anarlog's bundled model (`841645a9…`) and
    sherpa-onnx's release file (`e9848563…`) at cosine similarity 1.000000 on
    the fixtures (`tests/model_parity.rs`); Deino computes the filterbank's
    per-utterance mean normalisation itself, so sherpa's missing CMN metadata
    does not matter.
- **`knf-rs` is vendored** under `crates/third_party/knf-rs` (MIT, with
  kaldi-native-fbank 1.20.1, Apache-2.0) and compiled with `cc`: no git
  dependency, no CMake, no bindgen.
- **Parameters by meeting length** (`policy`): step 2 s (3 s past 30 min),
  threshold 0.7045, `min_cluster_speech` 3 s under 2 min else 12 s; calendar
  counts are exact when every invitee confirmed and the clip is at least
  2 min long, else an upper bound. Then OpenWhispr's phantom pruning and
  collapse detection.
- **The stage's rules live in `deino-core::diarization`** (`DiarizeService`,
  `plan_channels`, `attribute`) behind a `TrackDiarizer` port that
  `deino-diarize::track::OnnxTrackDiarizer` implements, so the rules are
  tested with fakes and the core stays free of native dependencies.
  Speakers keep their centroid, embedding model and speech time in new
  nullable `speakers` columns; segments are re-attributed (and split at
  speaker changes) in one transaction through `SegmentWriter::reattribute`.

## Alternatives

- **sherpa-onnx** (official crate): no centroids, `num_clusters` loses
  speakers, and its static onnxruntime silently replaces `ort`'s (losing Core
  ML). Rejected by the spike.
- **Hosting anarlog's blobs ourselves:** Deino has no server (§1). The
  publishers' files are equivalent and revision-pinned.
- **A pure-Rust filterbank:** would have to match Kaldi bit for bit; vendoring
  the reference implementation is cheaper and exact.
- **Padding embeddings into one batch:** changes the statistics pooling, so
  results drift; rejected for equal-length grouping.

## Consequences

- DER 9–12 % with bounds on the 3–5 speaker clips, RTF 0.012–0.017 at load
  about 20 on an M4 Max (`docs/perf/diarization.md`): a 60-minute meeting in
  about 45 s.
- The workspace's `ort` now has the `coreml` feature on macOS. The
  `-fapple-link-rtlib` link argument anarlog sets is not needed with pyke's
  rc.10 binaries (the app and every test binary link).
- Core ML compiles models into a cache: the caller passes
  `RuntimeConfig::accelerator_cache`. Diarizers are built one at a time (the
  pipeline runs one stage at a time), so there is no process-wide compile
  lock.
- Thresholds were measured on synthetic voices. Re-tune on AMI dev and on
  real meetings before changing them.
- The stage: `deino-pipeline`'s `DiarizeStage` (order 30) runs
  `DiarizeService`; the app builds `OnnxTrackDiarizer` over the STT models'
  `HttpByteSource` (`composition/diarize.rs`). Missing models are retried and
  then skipped on the last attempt, so notes never wait for speakers; a meeting
  without its track file is skipped.
- Follow-ups: move the downloads to `deino-net`; attribute WeSpeaker in the About screen; CAP-02 drives
  `online::LiveSpeakerTracker`; CAP-04 uses `voiceprint` and the stored
  centroids.
