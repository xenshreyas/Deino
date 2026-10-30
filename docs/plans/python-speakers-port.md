# Porting the Python speaker engine to Rust

The owner's opt-in Python speaker engine (`crates/deino-diarize/python/speakers.py`,
installed by `scripts/setup-python-speakers.sh`, used through
`deino_diarize::python::PythonEngine`) fixed who-said-what on real calls
(Oct 2026: the "Quesal, Paper-based exams, Interview Jordon" meeting, where
13:17, 17:04 and 0:50 were attributed to the wrong person). It is a stopgap:
it needs Python, about 2–3 GB of packages, and downloads them outside
`deino-net`. Each piece below is replaced in Rust, one at a time, and checked
against the Python output on that meeting before the Python piece goes.

| Piece | Python today | Rust replacement | Check |
|---|---|---|---|
| Segmentation | Community-1 segmentation (pyannote) | **Nothing to port:** Community-1's segmentation model is segmentation-3.0, which Deino already runs (exported, their outputs differ by at most 2e-5). On the reference call Deino's current engine is already at 3.2% DER against the Python engine (2.1% missed, 0.5% false alarm, 0.6% confusion; 0.25 s collar; `diarize-bench --segmentation`), with one 3-second phantom voice | frame-level activations match within 1e-3 |
| Voice embeddings | WeSpeaker ResNet34-LM via pyannote | already in Deino (`EMBEDDING_MODEL_ID`, Core ML); centroids match the Python ones at cosine 0.996. Overlapped speech is already kept out: a window's embedding uses only the frames where that voice speaks alone (`pipeline/scan.rs`), centroids average only those (`pipeline/labels.rs`), and enrolment spans cut overlaps (`deino-core::voiceprints::spans`) | per-voice cosine ≥ 0.99 |
| Clustering | PLDA + VBx (Community-1) | **Done:** `deino_diarize::vbx` (AHC at 0.6, PLDA, VBx with Fa 0.07 and Fb 0.8, constrained assignment), the app's default when the PLDA is bundled. Matches pyannote to 1e-6 on a synthetic case (`tests/vbx_reference.rs`); on the reference call the same 2 voices and 99.2% of words with the same speaker as the Python engine (98.7% before VBx, which gave the opening to the wrong person) | same voice count and ≥ 98% same turn labels on the meeting |
| Exclusive turns | `exclusive_speaker_diarization` | **Done:** with the PLDA bundled, reconstruction gives each frame to the most-heard speaker wherever the rounded mean count of talkers is at least one (`pipeline/reconstruct.rs`), at a 2 s step (`policy::community_config`). On the reference call: 99.27% of words on the same speaker as the Python engine, the same 10 lines split, the opening right; the rest are split points a word or two apart and one-word replies | turns within 50 ms |
| Word aligner | wav2vec2-base-960h CTC + Viterbi in numpy | **Done:** `deino_diarize::align` (`OnnxWordAligner`), the default without the Python engine. On the reference call's 5,492 words: with the fp32 export every word matches Python; with the bundled int8 model the median gap is 0 ms and 97% of starts are within 20 ms (`align-bench`) | word times within 40 ms |

## Model files ship inside the app

Deino is used by the owner's friends (macOS only), and everything runs on
the Mac. Model files are bundled in the app instead of downloaded, so no
login, no first-run download and no network are needed:

| File | Source | Licence | Size |
|---|---|---|---|
| WeSpeaker ResNet34-LM embedding | the one Deino already uses | CC-BY-4.0 | ~26 MB |
| PLDA and x-vector transform, precomputed (`community-1-vbx.f64`) | Community-1 | CC-BY-4.0 | 0.4 MB |
| wav2vec2-base-960h aligner (ONNX, MatMul int8) | facebook/wav2vec2-base-960h | Apache-2.0 | 122 MB |

Done: `scripts/export-speaker-models.py` makes them into
`frontend/src-tauri/resources/speakers/` (Git LFS); each is pinned in
`deino_diarize::models::bundled`, bundled as the `speakers/` resource and
credited in `THIRD_PARTY_NOTICES.md`. The ONNX engine reads its two models
from there instead of downloading them. The app grows by about 160 MB.
Quantizing every layer of the aligner to int8 (95 MB) broke it: on a real
meeting it agreed with PyTorch on 29% of speech frames; MatMul-only, per
channel, agrees on 97%.

The Python engine's answer on one real call (speaker turns and word times,
no text) is kept as the port's reference in
`crates/deino-diarize/tests/reference/`, on the owner's Mac only
(gitignored): it comes from a private recording.

When every row is done, delete `python/`, `src/python.rs`, the setup script and
this plan, and make the Rust path the only one.
