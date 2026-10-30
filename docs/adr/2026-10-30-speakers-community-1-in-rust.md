# Speakers: pyannote Community-1, in Rust, inside the app

Status: accepted (2026-10-30). Amends ADR 0020 (local speaker diarization).

**Context.** On real calls Deino's speaker engine (segmentation-3.0,
WeSpeaker, centroid-linkage clustering, majority-vote turns) gave lines to
the wrong person, and lines that changed speaker partway stayed whole,
because Whisper-style engines time only whole lines. For two weeks the
owner ran pyannote's Community-1 pipeline and a wav2vec2 word aligner
through Python instead (an opt-in engine under the app's data folder).
Who-said-what became right, but it needed Python, about 3 GB of packages
and a Hugging Face login, so friends could not use it.

**Decision.** The Python engine's pieces were ported to Rust in
`deino-diarize`, each checked against the Python output on one private
72-minute, two-voice call before it shipped. The Python engine is gone.

| Piece | Rust | Check against the Python engine |
|---|---|---|
| Segmentation | none needed: Community-1's segmentation model is segmentation-3.0 (outputs within 2e-5) | – |
| Embeddings | WeSpeaker ResNet34-LM, as before; overlapped speech already excluded | centroids at cosine 0.996 |
| Clustering | `vbx`: AHC at 0.6, PLDA, VBx (Fa 0.07, Fb 0.8), constrained assignment | pyannote's own numbers to 1e-6 on a synthetic case (`tests/vbx_reference.rs`) |
| Turns | one speaker per frame, at a 2 s step (`policy::community_config`) | same 10 lines split; 99.27% of words on the same speaker |
| Word times | `align`: wav2vec2-base-960h CTC forced alignment, int8 MatMul weights | fp32 export identical on every word; int8 median 0 ms, 97% within 20 ms (`align-bench`) |

The models ship inside the app (`speakers/` among its resources, Git LFS,
pinned in `models::bundled`, made by `scripts/export-speaker-models.py`):
nothing is downloaded and no login is needed. Community-1's PLDA ships
precomputed (`community-1-vbx.f64`), so Deino needs no linear-algebra
library.

**Consequences.** Everyone gets Community-1-quality speakers with nothing to
install. The app is about 155 MB larger. Speaker detection takes about
80 s on a 72-minute call (the 2 s step; 3 s took 34 s but added stray
splits). What still differs from the Python engine is small: split points
a word or two apart and one-word replies. `tests/speaker_ab.rs` repeats the
comparison on any local recording.
