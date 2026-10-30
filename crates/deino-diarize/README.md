# deino-diarize

## Role

Local speaker diarization (CAP-03, ADR 0020): who spoke when on one recorded
track, with a 256-d centroid per speaker. A port of anarlog's
`pyannote-local`, `embedding`, `onnx` and `voiceprint` crates (MIT):
pyannote segmentation-3.0 on the CPU, WeSpeaker ResNet34-LM embeddings on
Core ML, centroid-linkage clustering under calendar speaker bounds.

## Owns

- `Diarizer` (`pipeline`): batched segmentation, per-window filterbanks
  shared by local speakers, equal-length embedding batches, clustering,
  known-voiceprint merging, majority-vote reconstruction.
- `policy`: parameters by meeting length (step, `min_cluster_speech`,
  bounds); `prune`: phantom pruning and collapse detection (OpenWhispr).
- `models`: the pinned model files (`CATALOG`, `EMBEDDING_MODEL_ID`) and the
  `ModelStore` that downloads them through `deino-core`'s verified downloader
  (`ByteSource` port).
- `track::OnnxTrackDiarizer`: `deino-core`'s `TrackDiarizer` port (decode the
  finalized track with `deino-audio`, run, prune).
- `python::PythonEngine`: the owner's opt-in Python engine (pyannote
  Community-1 and a wav2vec2 word aligner, `python/speakers.py`), used when
  `<app data>/python-diarization/` is installed by
  `scripts/setup-python-speakers.sh`; a stopgap until
  `docs/plans/python-speakers-port.md` is done.
- `online` (live clustering) for CAP-02, `metrics` (DER) and the `diarize-bench`
  harness (`docs/perf/diarization.md`).

## Does Not Own

- The stage's rules (channel planning, segment attribution, speaker rows):
  `deino-core::diarization`.
- Voiceprint matching (`pick_unique`, `complete_by_elimination`), enrolment
  spans and `identify_speakers` (CAP-04): `deino-core::voiceprints` and
  `deino-core::diarization::identify`. The known-voiceprint merge here
  takes its `KnownSpeaker`s from `TrackRequest::known`.
- Where the models live and which `ByteSource` fetches them: the app's
  composition root.
- The filterbank itself: `crates/third_party/knf-rs` (vendored).

## Invariants

- No dependency on `tauri` or `deino-net`; models arrive only through the
  `ByteSource` port and are installed only after their SHA-256 matches.
- Inputs are 16 kHz mono; every sample/frame/time conversion is in `units`.
- Centroids are L2-normalised and tagged with `EMBEDDING_MODEL_ID`, so
  vectors from different models are never compared.
- Batching never changes a result (tested in `tests/model_parity.rs`).
- Models load per run; nothing is held after a meeting is diarized.
- Logs carry counts and durations, never audio or transcript content.

## Tests

`cargo test -p deino-diarize` runs every unit test without models. The model
checks and the benchmark need the ONNX files; see `docs/perf/diarization.md`.
