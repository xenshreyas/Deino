#!/usr/bin/env bash
# Installs the owner's opt-in Python speaker engine (experimental):
# pyannote Community-1 for who spoke when, and a wav2vec2 aligner for word
# times. Deino uses it for the `diarize` stage whenever this folder exists;
# delete the folder to go back to the built-in engine. Needs `uv`, the
# Hugging Face terms of pyannote/speaker-diarization-community-1 accepted,
# and `hf auth login` once. About 2–3 GB.
set -euo pipefail

ROOT="${DEINO_PYTHON_ENGINE_DIR:-$HOME/Library/Application Support/com.xenshreyas.deino/python-diarization}"
REPO="$(cd "$(dirname "$0")/.." && pwd)"

command -v uv >/dev/null || { echo "setup-python-speakers: uv is required (https://docs.astral.sh/uv/)" >&2; exit 1; }

mkdir -p "$ROOT"
[ -x "$ROOT/.venv/bin/python" ] || uv venv --quiet --python 3.12 "$ROOT/.venv"
uv pip install --quiet --python "$ROOT/.venv/bin/python" \
  "pyannote.audio==4.0.7" "torch==2.15.0" "transformers==5.22.0" "numpy<3"
cp "$REPO/crates/deino-diarize/python/speakers.py" "$ROOT/speakers.py"

# Fetch both models now, so the first meeting does not wait for them.
"$ROOT/.venv/bin/python" -I - <<'PY'
from pyannote.audio import Pipeline
from transformers import Wav2Vec2ForCTC, Wav2Vec2Processor
Pipeline.from_pretrained("pyannote/speaker-diarization-community-1")
Wav2Vec2Processor.from_pretrained("facebook/wav2vec2-base-960h")
Wav2Vec2ForCTC.from_pretrained("facebook/wav2vec2-base-960h")
PY

echo "Installed the Python speaker engine in $ROOT."
echo "Restart Deino to use it; delete that folder to turn it off."
