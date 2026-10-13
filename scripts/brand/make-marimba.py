"""Renders Deino's notification sound, `frontend/src-tauri/sounds/marimba.wav`.

The same two-note marimba as `docs/mockups/notification-sounds.html`: C5 then
G5 90 ms later, each a sine with a fast attack and an exponential decay, plus
a short fourth-harmonic "mallet" partial. Mono, 48 kHz, 16-bit; deterministic.

    python3 -I scripts/brand/make-marimba.py
"""

import math
import struct
import wave
from pathlib import Path

RATE = 48_000
LENGTH_S = 0.5
GAIN = 0.6  # the mockup's default volume
OUT = Path(__file__).resolve().parents[2] / "frontend/src-tauri/sounds/marimba.wav"


def envelope(t: float, attack: float, decay: float, peak: float) -> float:
    """Linear attack to `peak`, then an exponential fall to 1e-4 at `decay`."""
    if t < 0 or t > decay:
        return 0.0
    if t < attack:
        return peak * t / attack
    return peak * (1e-4 / peak) ** ((t - attack) / (decay - attack))


def sample(t: float) -> float:
    """The sound at time `t`: two notes, each with its mallet partial."""
    total = 0.0
    for freq, start in ((523.25, 0.0), (783.99, 0.09)):
        local = t - start
        total += envelope(local, 0.002, 0.35, 0.28) * math.sin(2 * math.pi * freq * local)
        total += envelope(local, 0.001, 0.08, 0.08) * math.sin(2 * math.pi * freq * 4 * local)
    return total * GAIN


def main() -> None:
    frames = int(RATE * LENGTH_S)
    data = b"".join(
        struct.pack("<h", max(-32767, min(32767, round(sample(i / RATE) * 32767))))
        for i in range(frames)
    )
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(OUT), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(data)
    print(f"{OUT} ({len(data)} bytes of audio)")


if __name__ == "__main__":
    main()
