# Spike: diarization and voiceprint engine (CAP-03, CAP-04)

**Setup:** 2026-10-09 · M4 Max 64 GB.
- **Code:** `~/Documents/GitHub/deino-spikes/diarization/` (crates `engine-anarlog`, `engine-sherpa`, `link-test`; `scripts/`).
- **Raw results:** `results/runs.jsonl`, `results/voiceprints.json`, `results/e2e/`.
- **Model pins:** `models/SHA256SUMS`.

## TL;DR (decided)

- **Engine:** port **anarlog `pyannote-local` + `embedding` + `voiceprint`** (MIT) on Deino's existing `ort =2.0.0-rc.10`.
  - Load the **embedding model through the CoreML execution provider** (NeuralNetwork format, CPU + Neural Engine). Its DER is bit-identical to CPU at about 10× less CPU.
  - Keep segmentation on CPU.
  - **Always pass calendar speaker bounds.**
- **Accuracy:** with a known count, DER was **8.3–11.7 % on every fixture**, including a 57-minute meeting, and the speaker count was always right.
  - sherpa-onnx's `num_clusters` loses speakers to outlier clusters (25–50 % DER).
  - With a threshold only, the two engines are about equal.
- **Speed:** RTF **0.023** on 57 minutes (step 3 s, CoreML, measured at load 26–28), so about 80 s.
  - Expect about 0.04–0.05 on an M1. **Batch the per-window embeddings** into one `ort` run to reach the CAP-03 target of 0.033.
- **Embedding model:** **WeSpeaker ResNet34-LM** (256-d, **CC-BY-4.0, attribution required**).
  - **CAM++ is unusable** (60 %+ DER, 11–24 % EER).
  - TitaNet-small is acceptable but weaker.
- **sherpa pitfall:** sherpa ships WeSpeaker without CMN, so the embeddings collapse (same speaker 0.96 vs different 0.85).
  - The fix is the ONNX metadata entry `feature_normalize_type=global-mean`.
  - Moot, because Deino uses anarlog.
- **Linking:**
  - Static sherpa + `ort` link and run, but sherpa's onnxruntime 1.28.2 silently replaces pyke's 1.22.0, and `ort`'s CoreML provider is lost.
  - Engine A avoids this. If sherpa is ever added, use its `shared` feature.
- **Don't trust TTS-tuned thresholds.**
  - A's 0.5 over-splits real voices. **Keep anarlog's AMI-tuned 0.7045** until it is re-tuned on real meetings.
  - `min_cluster_seconds = 12` collapses clips under about 1 minute, so use 3 s for clips under 2 minutes.

## Parameters by meeting length (Engine A)

| Meeting | step | bounds | threshold (no bounds) | `min_cluster_seconds` | expected RTF (M4 Max, CoreML) |
|---|---|---|---|---|---|
| < 2 min | 2 s | `max` = attendees − 1 (system track) | 0.7045 | 3 | about 0.02 + 0.4 s load |
| 2–30 min | 2 s | `exact(n)` if the calendar is trusted, else `max` | 0.7045 (re-tune on AMI dev) | 12 | 0.015–0.024 |
| > 30 min | 3 s (the 1,800-anchor cap takes over past about 90 min) | as above | as above | 12 | about 0.023 |

Step 5 breaks majority voting; don't use it.

## Voiceprints (CAP-04), WeSpeaker r34

**Measured similarity:**

| Level | same speaker | different speaker | EER |
|---|---|---|---|
| Segment | 0.835 ± 0.051 | 0.167 ± 0.135 (p95 0.50) | 0.1 % |
| Speaker (3-span centroid) | 0.902 ± 0.024 (min 0.824) | 0.178 (max 0.662) | 0 % |

**Enrolment:**
- Use each person's **3 longest clean spans**: overlap-free, at least 1.5 s each, middle 10 s.
- Match by the **max over up to 5 exemplars** per person (as anarlog does).

**Matching rules:**

| Decision | Rule |
|---|---|
| Auto-assign | **≥ 0.72** (or ≥ 0.77 for non-attendees), plus **margin ≥ 0.08** and **mutual best**. Prefer calendar attendees, and complete the rest by elimination. |
| Suggest | **0.55–0.72** |
| Live online clustering | join at ≥ 0.60 with a 0.05 margin, for single 1.5–3 s utterances |

- **Both safeguards are needed.** End to end, a phantom split cluster contaminated with another speaker matched the wrong person at 0.81. Mutual-best and the margin together rejected it.
- **Enrolment errors propagate.** A speaker merge during enrolment (seen with sherpa) produced later false accepts. Only enrol from a confirmed rename-all.
- **Unknown voice** (never enrolled): best score 0.35–0.38, so it correctly stays unmatched.
- **Calibration:** keep the thresholds in typed settings and calibrate on real dogfood meetings. Real cross-session scores will be lower than TTS scores.

**Post-processing to add:**
- phantom-speaker pruning (OpenWhispr rules);
- a duration-scaled `min_cluster_seconds`;
- known-voiceprint cluster merging (anarlog `apply_known_speakers`).

## Licences

| Artefact | Licence / note |
|---|---|
| anarlog crates | MIT © Fastrepl. Add a `THIRD_PARTY_NOTICES.md` entry. `enterprise/` was not used. |
| `knf-rs` | MIT; kaldi-native-fbank is Apache-2.0. **Vendor it**, since it is a git dependency today. |
| pyannote segmentation-3.0 | **MIT** © CNRS. |
| WeSpeaker ResNet34-LM | **CC-BY-4.0**. Attribution is required in `THIRD_PARTY_NOTICES.md` and the About screen. |
| sherpa-onnx static archive | Bundles espeak-ng (**GPL-3.0**). Only the `*-no-tts-lib` archives are acceptable, and sherpa isn't used anyway. |
| sherpa reverb-diarization models | Non-commercial; avoid. |

## sha256 pins

| Model | sha256 |
|---|---|
| anarlog segmentation (pyannote seg-3.0, onnx-community export) | `057ee564753071c0b09b5b611648b50ac188d50846bff5f01e9f7bbf1591ea25` |
| anarlog embedding (WeSpeaker ResNet34-LM) | `841645a9b5109369858ca89695e8b23ecfd4486b30bdd5cb3f99fef8d1c29205` |
| sherpa `wespeaker_en_voxceleb_resnet34_LM.onnx` | `e9848563da86f263117134dfd7ad63c92355b37de492b55e325400c9d9c39012` |

Anarlog's models are committed blobs with no download source, so Deino needs a model store download: host them, or use the HF onnx-community segmentation export plus the sherpa WeSpeaker file with CMN metadata.

## Code that worked (Engine A)

```rust
engine_anarlog::onnx::set_intra_threads(4);
let models = ModelPaths { segmentation: "models/anarlog-segmentation.onnx".into(),
                          embedding: "models/anarlog-embedding.onnx".into() };
let config = DiarizationConfig { step_seconds: 3.0, ..DiarizationConfig::default() };
let mut diarizer = Diarizer::new(config, &models)?;   // CoreML via feature `coreml`
let request = DiarizeRequest { bounds: SpeakerBounds::exact(4), ..DiarizeRequest::default() };
let result = diarizer.diarize(&mut samples.as_slice(), &request)?;
// result.segments {start,end,speaker}; result.speakers[i].centroid: Vec<f32> (256-d)
```

## Gaps

- **No real-meeting RTTM was used** (AMI and VoxConverse weren't authorised downloads). Run anarlog's `diarization-eval` on AMI dev to set the default threshold and confirm the voiceprint thresholds.
- **Timings were taken on an overloaded machine.** Re-time on an idle M1.
- **All DER fixtures are synthetic macOS `say` voices.**
