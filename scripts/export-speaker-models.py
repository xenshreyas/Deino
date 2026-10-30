"""Exports the speaker models Deino bundles (docs/plans/python-speakers-port.md).

Writes into frontend/src-tauri/resources/speakers/ (tracked with Git LFS):

- community-1-segmentation.onnx: pyannote Community-1's segmentation model
  (CC-BY-4.0), input `input_values` [batch, 1, samples] at 16 kHz, output
  `logits` [batch, frames, 7] (powerset log-probabilities), like the
  onnx-community export of segmentation-3.0 that Deino already runs.
- community-1-plda.npz, community-1-xvec-transform.npz: Community-1's PLDA
  arrays, copied as they are (plain NumPy arrays).
- wav2vec2-base-960h-int8.onnx: facebook/wav2vec2-base-960h (Apache-2.0),
  input `input_values` [batch, samples] (zero mean, unit variance), output
  `logits` [batch, frames, 32], its MatMul weights quantized to int8 per
  channel (quantizing the convolutions too wrecks it: on a real meeting the
  all-int8 model agreed with PyTorch on 29% of speech frames, this one 97%).
- pyannote-segmentation-3.0.onnx and wespeaker-voxceleb-resnet34-lm.onnx,
  the models Deino downloads today, copied from --existing.

Each export is checked against PyTorch before it is written. Needs the
Community-1 terms accepted on Hugging Face and `hf auth login`, plus
pyannote.audio, torch, transformers, onnx and onnxruntime. Run:

    python scripts/export-speaker-models.py --existing <app data>/models/diarization
"""

import argparse
import hashlib
import pathlib
import shutil

import numpy as np
import onnxruntime as ort
import torch
from huggingface_hub import hf_hub_download
from onnxruntime.quantization import QuantType, quantize_dynamic
from pyannote.audio import Model
from transformers import Wav2Vec2ForCTC

COMMUNITY = "pyannote/speaker-diarization-community-1"
COMMUNITY_REVISION = "3533c8cf8e369892e6b79ff1bf80f7b0286a54ee"
WAV2VEC2 = "facebook/wav2vec2-base-960h"
WAV2VEC2_REVISION = "22aad52d435eb6dbaf354bdad9b0da84ce7d6156"
OPSET = 17
REPO = pathlib.Path(__file__).resolve().parent.parent
OUT = REPO / "frontend/src-tauri/resources/speakers"


def run(path: pathlib.Path, inputs: np.ndarray) -> np.ndarray:
    session = ort.InferenceSession(str(path), providers=["CPUExecutionProvider"])
    return session.run(["logits"], {"input_values": inputs})[0]


def export_segmentation() -> None:
    model = Model.from_pretrained(COMMUNITY, subfolder="segmentation", revision=COMMUNITY_REVISION)
    model.eval()
    # Reference outputs come first: exporting can leave the module changed.
    check = torch.randn(3, 1, 80_000)
    with torch.no_grad():
        expected = model(check).numpy()
    sample = torch.randn(2, 1, 160_000)
    path = OUT / "community-1-segmentation.onnx"
    torch.onnx.export(
        model, (sample,), str(path), input_names=["input_values"], output_names=["logits"],
        dynamic_axes={"input_values": {0: "batch", 2: "samples"}, "logits": {0: "batch", 1: "frames"}},
        opset_version=OPSET, dynamo=False,
    )
    error = np.abs(run(path, check.numpy()) - expected).max()
    assert error < 1e-3, f"segmentation differs from PyTorch by {error}"
    print(f"segmentation: max difference {error:.2e}")


def export_plda() -> None:
    for name in ("plda", "xvec_transform"):
        source = hf_hub_download(COMMUNITY, f"plda/{name}.npz", revision=COMMUNITY_REVISION)
        shutil.copyfile(source, OUT / f"community-1-{name.replace('_', '-')}.npz")


def export_aligner() -> None:
    model = Wav2Vec2ForCTC.from_pretrained(WAV2VEC2, revision=WAV2VEC2_REVISION)
    model.eval()
    check = torch.randn(1, 48_000)
    with torch.no_grad():
        expected = model(check).logits.numpy()
    full = OUT / "wav2vec2-base-960h.fp32.onnx"
    sample = torch.randn(1, 32_000)
    torch.onnx.export(
        LogitsOnly(model), (sample,), str(full), input_names=["input_values"],
        output_names=["logits"],
        dynamic_axes={"input_values": {0: "batch", 1: "samples"}, "logits": {0: "batch", 1: "frames"}},
        opset_version=OPSET, dynamo=False,
    )
    path = OUT / "wav2vec2-base-960h-int8.onnx"
    error = np.abs(run(full, check.numpy()) - expected).max()
    assert error < 1e-2, f"aligner differs from PyTorch by {error}"
    quantize_dynamic(
        str(full), str(path), op_types_to_quantize=["MatMul"], per_channel=True,
        weight_type=QuantType.QInt8,
    )
    full.unlink()
    print(f"aligner: max difference {error:.2e} before quantizing")


class LogitsOnly(torch.nn.Module):
    """The CTC model returning its logits tensor, for the exporter."""

    def __init__(self, model: Wav2Vec2ForCTC) -> None:
        super().__init__()
        self.model = model

    def forward(self, input_values: torch.Tensor) -> torch.Tensor:
        return self.model(input_values).logits


def copy_existing(folder: pathlib.Path) -> None:
    for name in ("pyannote-segmentation-3.0.onnx", "wespeaker-voxceleb-resnet34-lm.onnx"):
        shutil.copyfile(folder / name, OUT / name)


def manifest() -> None:
    for path in sorted(OUT.glob("*")):
        if path.suffix in {".onnx", ".npz"}:
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            print(f"{path.name}  {path.stat().st_size}  {digest}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--existing", type=pathlib.Path, required=True,
                        help="the folder holding Deino's downloaded segmentation and WeSpeaker models")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    torch.manual_seed(0)
    export_segmentation()
    export_plda()
    export_aligner()
    copy_existing(args.existing)
    manifest()


if __name__ == "__main__":
    main()
