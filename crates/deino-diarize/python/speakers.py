"""Deino's experimental Python speaker engine (owner opt-in, see README.md).

Two commands over 16 kHz mono float32 PCM written by Deino:

  diarize --pcm F --out F [--min-speakers N] [--max-speakers N]
      pyannote Community-1: who spoke when (exclusive turns, one voice at a
      time) and one WeSpeaker ResNet34-LM embedding per voice, the model
      Deino's own diarizer and saved voice profiles use.

  align --pcm F --lines F --out F
      wav2vec2 CTC forced alignment: a start and end for every word of each
      line, so lines can be split where the voice changes.

Everything runs on this Mac; models come from the Hugging Face cache. JSON
out only; nothing is printed but errors.
"""
import argparse
import json
import re

import numpy as np

SAMPLE_RATE = 16_000
DIARIZER = "pyannote/speaker-diarization-community-1"
ALIGNER = "facebook/wav2vec2-base-960h"
PAD_MS = 300


def read_pcm(path):
    return np.fromfile(path, dtype="<f4")


def device(torch):
    return torch.device("mps") if torch.backends.mps.is_available() else torch.device("cpu")


def diarize(args):
    import torch
    from pyannote.audio import Pipeline

    pcm = read_pcm(args.pcm)
    pipeline = Pipeline.from_pretrained(DIARIZER)
    pipeline.to(device(torch))
    bounds = {}
    if args.min_speakers:
        bounds["min_speakers"] = args.min_speakers
    if args.max_speakers:
        bounds["max_speakers"] = args.max_speakers
    result = pipeline({"waveform": torch.from_numpy(pcm)[None], "sample_rate": SAMPLE_RATE}, **bounds)
    labels = list(result.speaker_diarization.labels())
    embeddings = np.asarray(result.speaker_embeddings) if result.speaker_embeddings is not None else []
    voices = {}
    for label, vector in zip(labels, embeddings):
        if np.all(np.isfinite(vector)) and np.linalg.norm(vector) > 0:
            voices[label] = [float(x) for x in vector / np.linalg.norm(vector)]
    turns = [
        {"start_ms": round(seg.start * 1000), "end_ms": round(seg.end * 1000), "speaker": label}
        for seg, _, label in result.exclusive_speaker_diarization.itertracks(yield_label=True)
        if label in voices
    ]
    write(args.out, {"turns": turns, "voices": voices})


def align(args):
    import torch
    from transformers import Wav2Vec2ForCTC, Wav2Vec2Processor

    pcm = read_pcm(args.pcm)
    lines = json.load(open(args.lines))
    processor = Wav2Vec2Processor.from_pretrained(ALIGNER)
    model = Wav2Vec2ForCTC.from_pretrained(ALIGNER).eval()
    vocab = processor.tokenizer.get_vocab()
    timed = []
    for line in lines:
        timed.append(align_line(pcm, line, processor, model, vocab, torch))
    write(args.out, {"lines": timed})


def align_line(pcm, line, processor, model, vocab, torch):
    words = line["text"].split()
    start = max(0, line["start_ms"] - PAD_MS)
    end = min(len(pcm) * 1000 // SAMPLE_RATE, line["end_ms"] + PAD_MS)
    chunk = pcm[start * SAMPLE_RATE // 1000: end * SAMPLE_RATE // 1000]
    tokens, owner = [], []
    for index, word in enumerate(words):
        letters = re.sub(r"[^A-Z']", "", word.upper())
        if not letters:
            continue
        if tokens:
            tokens.append(vocab["|"])
            owner.append(None)
        for letter in letters:
            tokens.append(vocab[letter])
            owner.append(index)
    times = [None] * len(words)
    if tokens and len(chunk) > 400:
        with torch.no_grad():
            inputs = processor(chunk, sampling_rate=SAMPLE_RATE, return_tensors="pt").input_values
            emission = torch.log_softmax(model(inputs).logits[0], dim=-1).numpy()
        if emission.shape[0] >= len(tokens):
            per_frame = (end - start) / emission.shape[0]
            for index, frames in frames_by_word(emission, tokens, owner, vocab["<pad>"]).items():
                times[index] = (start + min(frames) * per_frame, start + (max(frames) + 1) * per_frame)
    fill_gaps(times, line)
    return [{"text": w, "start_ms": round(t[0]), "end_ms": round(t[1])} for w, t in zip(words, times)]


def frames_by_word(emission, tokens, owner, blank):
    """CTC Viterbi alignment: the frame each token is emitted at, by word."""
    frames_count, count = emission.shape[0], len(tokens)
    trellis = np.full((frames_count + 1, count + 1), -np.inf)
    trellis[0, 0] = 0
    trellis[1:, 0] = np.cumsum(emission[:, blank])
    ids = np.array(tokens)
    for t in range(frames_count):
        trellis[t + 1, 1:] = np.maximum(trellis[t, 1:] + emission[t, blank], trellis[t, :-1] + emission[t, ids])
    j, t, at = count, frames_count, [0] * count
    while j > 0 and t > 0:
        stay = trellis[t - 1, j] + emission[t - 1, blank]
        move = trellis[t - 1, j - 1] + emission[t - 1, tokens[j - 1]]
        if move >= stay:
            at[j - 1] = t - 1
            j -= 1
        t -= 1
    words = {}
    for frame, index in zip(at, owner):
        if index is not None:
            words.setdefault(index, []).append(frame)
    return words


def fill_gaps(times, line):
    """Words the aligner could not place (numbers, symbols) sit between their neighbours."""
    known = [i for i, t in enumerate(times) if t]
    for i, t in enumerate(times):
        if t is None:
            before = max((k for k in known if k < i), default=None)
            after = min((k for k in known if k > i), default=None)
            a = times[before][1] if before is not None else line["start_ms"]
            b = times[after][0] if after is not None else line["end_ms"]
            times[i] = (a, max(a, b))


def write(path, value):
    with open(path, "w") as out:
        json.dump(value, out)


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    d = commands.add_parser("diarize")
    d.add_argument("--pcm", required=True)
    d.add_argument("--out", required=True)
    d.add_argument("--min-speakers", type=int)
    d.add_argument("--max-speakers", type=int)
    a = commands.add_parser("align")
    a.add_argument("--pcm", required=True)
    a.add_argument("--lines", required=True)
    a.add_argument("--out", required=True)
    args = parser.parse_args()
    diarize(args) if args.command == "diarize" else align(args)


if __name__ == "__main__":
    main()
