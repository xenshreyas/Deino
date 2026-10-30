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
| Segmentation | Community-1 segmentation (pyannote) | Community-1 segmentation weights exported to ONNX, run in `deino-diarize::segmentation` (same architecture as segmentation-3.0) | frame-level activations match within 1e-3 |
| Voice embeddings | WeSpeaker ResNet34-LM via pyannote | already in Deino (`EMBEDDING_MODEL_ID`, Core ML); centroids match the Python ones at cosine 0.996 | per-voice cosine ≥ 0.99 |
| Clustering | PLDA + VBx (Community-1) | port PLDA transform and VBx to `deino-diarize::clustering` (references: pyannote, FluidAudio) | same voice count and ≥ 98% same turn labels on the meeting |
| Exclusive turns | `exclusive_speaker_diarization` | one voice per frame in the reconstruction step | turns within 50 ms |
| Word aligner | wav2vec2-base-960h CTC + Viterbi in numpy | export to ONNX, run with `ort`; port `frames_by_word` and `fill_gaps` | word times within 40 ms |

## Model files ship inside the app

Deino is used by the owner's friends (macOS only), and everything runs on
the Mac. Model files are bundled in the app instead of downloaded, so no
login, no first-run download and no network are needed:

| File | Source | Licence | Size |
|---|---|---|---|
| Community-1 segmentation (ONNX) | pyannote/speaker-diarization-community-1 (gated on Hugging Face, so it cannot be fetched for users) | CC-BY-4.0 | ~6 MB |
| WeSpeaker ResNet34-LM embedding | the one Deino already uses | CC-BY-4.0 | ~26 MB |
| PLDA `plda.npz`, `xvec_transform.npz` as plain arrays | Community-1 | CC-BY-4.0 | < 1 MB |
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
