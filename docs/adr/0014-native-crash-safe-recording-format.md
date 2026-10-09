# 0014. Record each track as CMAF segments from AVAssetWriter into Deino-owned files

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** FND-05a (split of FND-05)

## Context

Deino records the microphone and system audio of meetings that can last
hours. The inherited code writes 30 s MP4 checkpoints and joins them with an
FFmpeg sidecar, so a crash can lose up to 30 s, and FFmpeg is a GPL-licensed
binary we want gone (PARITY_PLAN D11). FND-05 requires that `kill -9` lose
about 1 s at most, that power loss lose about 5 s, and that the microphone,
system and mixed files stay aligned within 50 ms.

The spike `docs/references/spike-native-recording.md` measured seven ways of
writing AAC natively on macOS under `kill -9` and torn writes.

## Decision

- **Format:** AAC-LC, 64 kbps, 48 kHz, mono, one file per track, as a
  fragmented MP4 (CMAF) with the `.m4a` extension.
- **Encoder:** `AVAssetWriter` in segment-output mode (`public.mpeg-4`
  content type, `MPEG4CMAFCompliant` profile, 1 s preferred segment
  interval). A delegate receives the init segment and one `moof` + `mdat`
  per second and Deino appends each with one `write(2)` to
  `audio/<track>.NNN.m4a.part`. Deino, not AVFoundation, owns the file, the
  fsync cadence and the count of durable frames.
- **Durability:** a persister thread snapshots the frames each file holds,
  `F_FULLFSYNC`s the files, then rewrites `recording.json` atomically (temp
  file, full fsync, rename, directory fsync) every 5 s. The manifest is
  typed (`RecordingManifest`) and versioned.
- **Timeline:** the sample index is time. Each track's PTS is its cumulative
  frame count over 48 000; gaps become silence and a late track is padded
  from the shared epoch, so the tracks are aligned by construction.
- **Threads:** capture callbacks write a preallocated `rtrb` ring only; one
  writer thread per track drains it every 10 ms into a 64 MiB backlog and
  feeds the encoder when it is ready.
- **Recovery:** at launch, `truncate_to_playable` cuts any torn tail back to
  the last whole fragment, the files are renamed to `.m4a`, and the manifest
  records the playable frames (`packets × 1024 − 2112`).
- **Finalize:** a single-segment track is passthrough-exported to a flat
  `mic.m4a` / `system.m4a` (no re-encode). `audio.mp4` is derived: both tracks
  are decoded with ExtAudioFile (priming trimmed), summed at −6 dB through a
  soft limiter and encoded. There is no third live encoder; the mix being a
  pure function of the recovered tracks is how "recovery for all three files"
  holds (`audio-capture.md` §5).

## Alternatives

- **ExtAudioFile → M4A:** no `moov` until close; unreadable after a kill.
- **ExtAudioFile → CAF:** the AAC packet table is written at close; 0 frames.
- **ExtAudioFile → ADTS:** ≤ 0.1 s lost, but Apple writes `0xFFF9` headers
  that symphonia rejects, there is no priming information and it is not M4A.
- **AVAssetWriter with `movieFragmentInterval`:** crash-safe, but
  AVFoundation owns the file, so Deino cannot control fsync or know which
  frames are durable.
- **5 s chunks with a manifest:** up to 4.6 s lost, 2.5× the size, and
  finalize must re-encode because each chunk adds 44 ms of priming.
- **FFmpeg fragmented MP4 (`audio-capture.md` §5's first plan):** works, but
  keeps the GPL sidecar D11 removes.

## Consequences

- Measured on this branch (debug build, machine load about 80–100):
  `kill -9` lost 0.18–0.58 s in 3 trials, and both recovered tracks open in
  `AVAudioFile`; clicks in `mic.m4a`, `system.m4a` and `audio.mp4` sit within
  1 ms (46 frames) of where they were played; about 2.7 % of one core per
  track including the mock sources (the spike measured 1.7 % in release).
- symphonia 0.5 ignores AAC priming, so anything decoding Deino's files with
  it (CAP-08 re-transcription, CAP-11 imports) must trim 2112 frames or use
  ExtAudioFile/AVFoundation.
- cidre a9587fa lacks `finishWritingWithCompletionHandler:`,
  `AVAssetExportSession` and three AudioToolbox setters, so the crate uses
  objc2 and a small FFI module for them. Revisit when cidre gains them.
- Multi-segment tracks (a new segment after an encoder restart) are spliced
  by decode and re-encode at finalize; nothing creates them yet.
- Revisit if WKWebView playback of the fragmented files proves unreliable
  (finalize already produces flat files) or if real power-loss testing shows
  losses above 5 s.
