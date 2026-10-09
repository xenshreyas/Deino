# knf-rs (vendored)

## Role

Kaldi log-mel filterbank features for the WeSpeaker speaker-embedding model
(`deino-diarize`). Vendored from knf-rs 0.3.2 (`thewh1teagle/pyannote-rs` at
`e23bd29`, MIT, `LICENSE-knf-rs`), which wraps kaldi-native-fbank 1.20.1
(Apache-2.0, `LICENSE-kaldi-native-fbank`); anarlog depended on it as a git
crate.

## Owns

- `kaldi-native-fbank/csrc/`: the upstream C++ sources the filterbank needs,
  unchanged (`log.cc` and the tests are left out, as upstream's CMake does
  without `KNF_ENABLE_CHECK`).
- `knfc/`: upstream's two-function C shim, unchanged.
- `src/knf_rs.rs`: the safe wrapper, rewritten for Deino: `compute_fbank`
  returns a typed `Fbank` (80 mel bins per 10 ms frame, per-utterance mean
  normalisation) or an `FbankError`.
- `build.rs`: compiles the C++ with `cc` at `-O2` (no CMake, no bindgen).

## Does Not Own

- Scaling, masking and batching the features for the model: `deino-diarize`.

## Invariants

- The numbers match upstream knf-rs exactly (16 kHz, 25 ms Povey windows,
  10 ms shift, no dither, `snip_edges`); `deino-diarize`'s model checks
  compare embeddings against anarlog's.
- The C++ sources stay verbatim, so a future update is a copy plus a review
  of the shim.
