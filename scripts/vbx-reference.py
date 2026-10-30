"""Writes the VBx reference case Deino's Rust port is tested against.

Synthetic x-vectors (three voices, seeded noise; no recordings) go through
pyannote.audio's own Community-1 clustering steps: centroid-linkage AHC on
unit vectors cut at the threshold, the PLDA transform, and VBx. The answers
land in crates/deino-diarize/tests/fixtures/vbx-case.json. Needs pyannote.audio
4.0.7 and the Community-1 PLDA on Hugging Face (terms accepted). Run:

    python scripts/vbx-reference.py
"""

import json
import pathlib

import numpy as np
from huggingface_hub import hf_hub_download
from pyannote.audio.core.plda import PLDA
from pyannote.audio.utils.vbx import cluster_vbx
from scipy.cluster.hierarchy import fcluster, linkage

COMMUNITY = "pyannote/speaker-diarization-community-1"
REVISION = "3533c8cf8e369892e6b79ff1bf80f7b0286a54ee"
THRESHOLD, FA, FB = 0.6, 0.07, 0.8
OUT = pathlib.Path(__file__).resolve().parent.parent / "crates/deino-diarize/tests/fixtures/vbx-case.json"


def main() -> None:
    rng = np.random.default_rng(7)
    voices = rng.normal(0, 1, (3, 256))
    counts, noise = [25, 18, 7], [0.4, 0.5, 0.7]
    x = np.vstack([v + rng.normal(0, s, (n, 256)) for v, n, s in zip(voices, counts, noise)])
    x = x[rng.permutation(len(x))]
    plda = PLDA(
        hf_hub_download(COMMUNITY, "plda/xvec_transform.npz", revision=REVISION),
        hf_hub_download(COMMUNITY, "plda/plda.npz", revision=REVISION),
    )
    normed = x / np.linalg.norm(x, axis=1, keepdims=True)
    ahc = fcluster(linkage(normed, method="centroid", metric="euclidean"), THRESHOLD, criterion="distance") - 1
    _, ahc = np.unique(ahc, return_inverse=True)
    fea = plda(x)
    gamma, pi = cluster_vbx(ahc, fea, plda.phi, Fa=FA, Fb=FB, maxIters=20)
    case = {
        "threshold": THRESHOLD, "fa": FA, "fb": FB,
        "embeddings": x.tolist(), "ahc": ahc.tolist(), "fea": fea.tolist(),
        "gamma": gamma.tolist(), "pi": pi.tolist(),
    }
    OUT.write_text(json.dumps(case))
    print(f"{len(x)} embeddings, {ahc.max() + 1} AHC clusters, kept {(pi > 1e-7).sum()} after VBx")


if __name__ == "__main__":
    main()
