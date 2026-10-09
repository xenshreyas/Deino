# Spike: FFmpeg-free crash-safe recording (FND-05), import decoding (CAP-11), clips (TRN-04)

**Environment:** 2026-10-08 · macOS 26.6.2 · M4 Max · Rust 1.92 · `cidre` git `a9587fa` (same rev as Deino) · `symphonia` 0.5.5 · `objc2` 0.6.4 / `objc2-av-foundation` 0.3.2.

**Code:**
- `src/`, crate `native-recording`, about 1.6k lines.
- Raw results: `out/kill/results.jsonl`, `out/imports.jsonl`, `out/truncate.jsonl`, `out/cpu_results.txt`.

## TL;DR
- **FND-05:** use **AVAssetWriter in segment-output mode** (CMAF profile, 1 s segments).
  - A delegate receives each segment's bytes, and Deino appends them to its own `mic.NNN.m4a.part` / `system.NNN.m4a.part`.
  - Format: AAC-LC, 64 kbps, 48 kHz, mono, fragmented MP4.
  - **`kill -9` loss: 0.03–0.84 s (6/6 trials).** After a kill, `afinfo`, symphonia and AVAudioFile all read the file without repair.
  - **Torn tail** (simulated power loss): fixed by `src/recover.rs`, a box scanner that truncates to the last complete `moof`+`mdat`. After that, all readers succeed (12/12).
  - **Cost:** about 1.7 % of one core per track and 29.9 MB per hour. No FFmpeg.
- **Rejected:**

  | Option | Why |
  |---|---|
  | ExtAudioFile → M4A | No `moov` after a kill, so the file is unreadable |
  | ExtAudioFile → CAF | 0 frames: the AAC `pakt` is written only at close |
  | 5 s chunks | Up to 4.6 s lost, 2.5× the size, and 13 s per hour to re-encode at finalize |
  | ExtAudioFile → ADTS | ≤0.1 s lost, **but symphonia 0.5.5 can't read Apple ADTS (`0xFFF9`)** |
- **CAP-11:** symphonia + `symphonia-adapter-libopus` 0.2 (MIT/Apache, bundles BSD-3 libopus) decodes every fixture, including Opus in MKV/WebM.
  - Add the `alac` and `aiff` features (and `caf`).
  - Fall back to **AVAssetReader** for Apple ADTS and AAC-in-CAF.
  - AVFoundation cannot open MKV or WebM.
- **TRN-04:** `AVAssetExportSession` (Passthrough preset) with a `timeRange` cuts sample-exactly (0 ms error via AVFoundation) in 11 ms.
- **Trap:** symphonia 0.5.5 **ignores AAC priming**, so every AAC decode is 2112 frames (44 ms) late.
  - Decode Deino's own files with AVFoundation/ExtAudioFile, or trim 2112 frames after a symphonia decode.
  - Re-check on symphonia 0.6.x.

## 1. Method
- **Signal:** `say` speech plus a 440 Hz sine at −20 dB; 48 kHz mono f32; 130 s long.
- **Writer:** 10 ms blocks paced in real time; `flush()` every 1 s, `sync()` (F_FULLFSYNC) every 5 s.
- **Kill test:** SIGKILL at a random time in [60, 120) s. Then check afinfo, symphonia and AVAudioFile.
  - **Loss** = appended − decoded seconds.
  - **Lag** is found by cross-correlation.
- **CPU:** `getrusage` over a 60 s run. **Size:** a 3600 s encode.

## 2. Results

### 2.1 Crash test (`kill -9`)
| Option | Trials | afinfo | symphonia (loss s) | AVAudioFile (loss s) | Tail lag sym/AVF (ms) |
|---|---|---|---|---|---|
| ExtAudioFile → M4A | 3 | 0/3 (no moov) | 0/3 | 0/3 | — |
| ExtAudioFile → CAF | 3 | opens, 0 s | 0/3 | 0/3 (0 frames) | — |
| ExtAudioFile → ADTS | 6 | 6/6 (duration off) | **0/6** | 6/6 (−0.09 to −0.01) | n/a / 44 |
| AVAssetWriter .m4a, movieFragmentInterval=1 s | 6 | 6/6 | 6/6 (0.08–0.60) | 6/6 (0.13–0.64) | 44 / 0 |
| AVAssetWriter .mov, movieFragmentInterval=1 s | 3 | 3/3 | 3/3 (0.10–0.59) | 3/3 (0.10–0.59) | 44 / 44 |
| **AVAssetWriter CMAF segments → own fMP4 .m4a** | 6 | **6/6** | **6/6 (−0.01–0.80)** | **6/6 (0.03–0.84)** | 44 / **0** |
| 5 s M4A chunks + manifest | 6 | 6/6 | 6/6 (0.05–4.55) | 6/6 (0.10–4.60) | 44 / 0 |

### 2.2 Cost (one track)
| Backend | CPU (% of one core) | Encode CPU per hour | Size per hour | Finalize, 1 h |
|---|---|---|---|---|
| ext-m4a | 0.85 % | 9.4 s | 29.4 MB | — |
| ext-adts | 0.93 % | 8.5 s | 29.8 MB | remux needed |
| avw-frag-m4a | 1.65 % | 25.3 s | 30.5 MB | none |
| **avw-cmaf** | **1.66 %** | 23.8 s | **29.9 MB** | optional passthrough export **1.3 s** |
| chunks | 0.95 % | 12.4 s | 73.2 MB | 13.2 s re-encode |

### 2.3 Torn writes (simulated power loss)
- A clean CMAF file was cut at 12 random offsets.
- **Before** repair: symphonia read **0/12** (a torn box breaks its probe); AVAudioFile read 12/12.
- **After** `truncate_to_playable`: both read **12/12**, and the frame counts matched.
- Under `kill -9` no tear occurs, because each ~8 KB segment is a single `write(2)`.

### 2.4 Why each option behaved as it did
- **ExtAudioFile M4A:** the `moov` is written only at dispose.
- **CAF:** only crash-tolerant for constant-size packets. AAC is VBR and needs `pakt`, which is written at close.
- **ADTS:**
  - It self-syncs, but Apple writes `0xFFF9` while symphonia only accepts `0xFFF1`.
  - It has no priming info and is not M4A.
  - It is a plan B only if Deino writes its own `0xFFF1` framing.
- **`movieFragmentInterval`:** works and is crash-safe (this amends the `audio-cap-zed.md` caution). However, AVAssetWriter owns the file, so Deino can't control fsync or account for durable frames.
- **CMAF delegate:** the same encoder, but Deino owns the writes and fsync and knows exactly what is on disk. The output is a standard fMP4 that every reader handles.
- **Chunks:**
  - Up to one chunk is lost.
  - Every chunk boundary adds an encoder discontinuity.
  - Concatenation would accumulate 44 ms of priming per chunk, so finalize must re-encode.

## 3. Recommended design for FND-05

### Format and writer
- **Format:** AAC-LC, 64 kbps, 48 kHz, mono; one file per track; **fragmented MP4 (CMAF)** with the `.m4a` extension; encoded in-process by AVAssetWriter.
- **Writer setup:**
  - `AVAssetWriter initWithContentType: public.mpeg-4`. `public.mpeg-4-audio` throws.
  - `outputFileTypeProfile = MPEG4CMAFCompliant`, `preferredOutputSegmentInterval = 1 s`, `initialSegmentStartTime = 0`, plus a delegate.
- **Input:**
  - `AVAssetWriterInput(audio, {AAC, 48000, 1, 64000}, sourceFormatHint = f32 mono)`, `expectsMediaDataInRealTime = true`.
  - PTS = cumulative frame index / 48000.
- **Delegate:**
  - Use `assetWriter:didOutputSegmentData:segmentType:segmentReport:`.
  - Append the bytes with one `write_all`.
  - Add the segment duration to `durable_frames`.
- **Threading:** append on the writer thread, never the real-time thread. Respect `isReadyForMoreMediaData` and feed the backlog.
- **Closing:** `markAsFinished`, then `finishWritingWithCompletionHandler` (not the deprecated sync call).

### Cadence
| What | When | Effect |
|---|---|---|
| Segment emitted and written | every 1 s | a `kill -9` loses ≤1 s |
| F_FULLFSYNC both `.part` files | every 5 s | power loss loses ≤5 s plus at most one torn fragment |
| `recording.json` atomic rewrite (tmp → F_FULLFSYNC → rename → fsync dir), recording durable frames | every 5 s and on events | the manifest never claims audio that isn't on disk |

### Layout
```
meetings/<id>/recording.json   {version, t0_host_ns, sample_rate, priming_frames: 2112, tracks[{track, segments[{file, first_frame, durable_frames, closed}]}], status}
meetings/<id>/audio/mic.001.m4a.part    fMP4: ftyp moov(mvex, elst 2112) (moof mdat)×N
meetings/<id>/audio/system.001.m4a.part
meetings/<id>/mic.m4a system.m4a audio.mp4   ← finalize_audio
```

### Recovery at launch
1. Run `truncate_to_playable` on every `.part` file. Playable frames = `packets × 1024 − 2112`.
2. Mark the segments closed, rename them, fsync the directory, and write the manifest.
3. Set the meeting to `processing` and enqueue `finalize_audio`.

### Finalize
- **One segment per track:** passthrough-export it to a flat `.m4a` (1.3 s per hour), or keep the fMP4.
- **Several segments:** decode each and re-encode into one M4A, inserting gap silence. `AVMutableComposition` + passthrough was not tested.
- **`audio.mp4`:** decode both tracks (priming trimmed), sum at −6 dB with a limiter, and encode.

### Alignment
Both tracks have identical 2112-frame priming, so they stay sample-aligned.

## 4. CAP-11: import decoding without FFmpeg
| File | Deino's symphonia today | symphonia `all` + libopus | AVAssetReader |
|---|---|---|---|
| aac.m4a | ✅ | ✅ (+44 ms) | ✅ |
| alac.m4a | ❌ (needs `alac`) | ✅ | ✅ |
| mp3 / wav / flac / vorbis.ogg | ✅ | ✅ | ✅ |
| aiff | ❌ (needs `aiff`) | ✅ | ✅ |
| opus.ogg / .opus / opus.caf | ❌ | ✅ (libopus, +`caf`) | ✅ |
| Apple ADTS (`0xFFF9`) | ❌ | ❌ | ✅ |
| AAC-in-CAF | ❌ | ❌ | ✅ |
| mp4 / mov / m4v video | ✅ | ✅ | ✅ |
| mkv (aac) | ✅ | ✅ | ❌ |
| mkv / webm with opus | ❌ | ✅ | ❌ |
| webm with vorbis | ✅ | ✅ | ❌ |

**Recommendation:**
1. Use symphonia first, with `alac`, `aiff` and `caf`, and register `symphonia_adapter_libopus::OpusDecoder` (version 0.2 targets symphonia 0.5).
2. On any probe or codec error, fall back to AVAssetReader, requesting 48 kHz mono f32.
3. MKV and WebM are symphonia-only.

**Untested:** AC-3/E-AC-3, HE-AAC v2 and very large files.

## 5. TRN-04: clips without FFmpeg
| Method | Time | Error read by AVF (start/dur) | Error read by symphonia |
|---|---|---|---|
| ExportSession AppleM4A + timeRange | 11 ms | 0 / 0 ms | −38.7 / +48 ms |
| ExportSession Passthrough + timeRange | 11 ms | 0 / 0 ms | −38.7 / +48 ms |
| AVAssetReader decode + ExtAudioFile encode | 95 ms | 0 / 0 ms | −44 / +48 ms |

**Recommendation:**
- Use the Passthrough preset, falling back to AppleM4A, with output type `AVFileTypeAppleM4A`.
- cidre `a9587fa` lacks `AVAssetExportSession`, so use objc2 (`src/clip.rs`).
- For video, use Passthrough with MPEG4 (not tested).

## 6. Pitfalls
1. symphonia 0.5.5 ignores AAC priming, so every decode is 44 ms late.
2. symphonia's ADTS reader requires `0xFFF1`.
3. symphonia can't decode AAC-in-CAF.
4. A torn fMP4 tail breaks symphonia's probe. Always run `truncate_to_playable` first.
5. Segment output only accepts the `public.mpeg-4` UTI.
6. Sync `finishWriting` is deprecated.
7. ExtAudioFile M4A reserves about 48 KB for `moov`.
8. cidre `a9587fa` lacks `ExtAudioFileSetProperty`/`AudioConverterSetProperty` (see `src/ffi.rs`) and `AVAssetExportSession`/`AVMutableComposition` (use objc2).

## 7. Code
See `src/encoder/cmaf.rs`, `src/encoder/asset_writer.rs`, `src/recover.rs`, `src/clip.rs`, `src/probe.rs` and `src/av_reader.rs`. All of it compiles.

## 8. Reproduce
```
cargo build --release
B=target/release/native-recording
$B gen-signal fixtures/speech48k.wav fixtures/signal130.wav 130
scripts/run_kill_matrix.sh 3
BACKENDS="ext-adts avw-frag-m4a avw-cmaf chunks" scripts/run_kill_matrix.sh 3 4
python3 scripts/summarize_kill.py
scripts/cpu.sh
$B truncate-test out/clean_cmaf130.m4a 12
scripts/make_import_fixtures.sh && $B imports fixtures/imports/*
$B clip passthrough out/clip/src.m4a out/clip/c.m4a 10 20 fixtures/signal60.wav
```

## Not covered
- Real power loss.
- WKWebView playback of the fMP4 (likely fine; or defragment at finalize).
- Two writers in one process.
- App Sandbox / hardened runtime.
