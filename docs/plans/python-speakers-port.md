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

When every row is done, delete `python/`, `src/python.rs`, the setup script and
this plan, and make the Rust path the only one.
