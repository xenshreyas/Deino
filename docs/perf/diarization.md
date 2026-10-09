# Diarization: accuracy and speed (CAP-03)

How Deino's local diarizer (`crates/deino-diarize`, ADR 0019) was measured,
and what it scored. Re-run it after any change to the models, the parameters
(`policy.rs`) or the post-processing (`prune.rs`).

## How to run

```bash
# The pinned models, by their store names, in <dir>:
#   pyannote-segmentation-3.0.onnx  wespeaker-voxceleb-resnet34-lm.onnx
cargo run -p deino-diarize --release --features bench --bin diarize-bench -- \
    --models <dir> fixtures
cargo run -p deino-diarize --release --features bench --bin diarize-bench -- \
    --models <dir> file <clip.wav|.m4a> <clip.rttm> --speakers <n>
# Model checks (parity with anarlog's bundled model, Core ML vs CPU, batching):
DEINO_DIARIZE_MODELS=<dir> cargo test -p deino-diarize --release \
    --test model_parity -- --ignored --nocapture --test-threads=1
```

The bench runs the production path: `policy::config_for` and `bounds_for`
from the clip length and the expected count, `Diarizer`, then
`track::post_process` (collapse check, phantom pruning). `--speakers n` is
what a fully confirmed calendar would give (exact past 2 minutes, an upper
bound below). Each run prints one JSON line.

**DER** follows pyannote.metrics (anarlog's `metrics.rs`): an optimal
one-to-one speaker mapping (Hungarian), scored on a 10 ms grid, with a 0.25 s
collar (half on each side of every reference boundary); the strict number
without a collar is reported too. **RTF** is processing time over audio time,
excluding the model load (0.3–0.7 s). Timings come with the 1-minute load
average: the machine is shared by parallel agents, so they are only
comparable at similar load (PARITY_PLAN §7.0).

## Results (2026-10-09, M4 Max 64 GB, Core ML, 4 intra-op threads)

**Multi-speaker clips with calendar bounds** (the spike's synthetic macOS
`say` meetings, `~/Documents/GitHub/deino-spikes/diarization/data/synth`):

| Clip | Length | Speakers (ref → found) | DER (0.25 s collar) | DER (no collar) | RTF | Load |
|---|---|---|---|---|---|---|
| `synth3_b` | 5.2 min | 3 → 3 | **10.9 %** | 15.2 % | 0.016 | 22.3 |
| `synth4_a` | 4.9 min | 4 → 4 | **11.2 %** | 15.3 % | 0.016 | 22.2 |
| `synth5_c` | 4.8 min | 5 → 5 | **11.7 %** | 14.8 % | 0.017 | 21.7 |
| `synth4_long` | 57.2 min | 4 → 4 | **9.4 %** | 12.6 % | **0.012** (42 s) | 20.0 |

**FND-04 fixtures with audio** (the first ~54 s of each meeting):

| Case | Bounds from the calendar | Speakers (ref → found) | DER |
|---|---|---|---|
| one-on-one, system track | at most 1 | 1 → 1 | 11.1 % (all missed speech at turn edges) |
| one-on-one, both tracks mixed | at most 2 | 2 → 2 | 7.0 % |
| standup, both tracks mixed | at most 5 | 3 → 3 | 4.8 % |
| standup, system track | at most 4 | 2 → 3 | 40.2 % |
| standup, system track, `--speakers 2` | at most 2 | 2 → 2 | 5.6 % |

The standup's system clip is the 54-second excerpt of a 7-minute call: only 2
of the 4 expected remote voices speak in it, so the calendar's upper bound of
4 lets the threshold split one voice. On the full meeting the bound is
correct; this is a property of the fixture clip, not of the stage.

**Without bounds** the AMI-tuned threshold under-counts synthetic voices
(`synth4_a`: 3 of 4 speakers, 25.5 % DER; `synth3_b`: 10.9 %), as the spike
found. Always pass calendar bounds.

**Acceptance:** DER ≤ 20 % on the multi-speaker clips with bounds (9–12 %);
RTF ≤ 0.035 (0.012–0.017 at load about 20, so a 60-minute meeting in about
45 s, against the 2-minute target).

## Model checks (`tests/model_parity.rs`)

| Check | Result |
|---|---|
| Pinned WeSpeaker (`7bb2f06e…`) vs anarlog's bundled model (`841645a9…`), 8 fixture utterances | min cosine 1.000000 |
| sherpa-onnx's release file (`e9848563…`) vs anarlog's | min cosine 1.000000 |
| Core ML vs CPU (pinned model) | min cosine 0.999998 |
| A batch of equal-length inputs vs single runs | identical (cosine > 0.9999) |
| A batch of 4 segmentation windows vs single windows | identical activity |

## Gaps

- No real-meeting reference (AMI, VoxConverse) yet; every number above uses
  synthetic voices. Re-tune the threshold on AMI dev before changing it.
- Timings were taken at load about 20, not on an idle M1.
