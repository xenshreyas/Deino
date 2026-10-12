# 0044. Convert imports to the recording format, and poll watched folders against a ledger

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** CAP-11

## Context

CAP-11 promotes audio import out of beta, accepts video (MP4, MOV, MKV,
WebM) and adds watched folders. The inherited import (`audio/import.rs`)
decoded with symphonia and the FFmpeg sidecar, ran its own transcription
and wrote the `transcripts` view, so imported meetings skipped the pipeline
and stayed `source = 'live'`. Since CAP-08 the pipeline transcribes a
meeting from its recording files (`final_transcription` over
`OfflineTranscriber`), and every later stage (diarization, clips, the
player) reads those files through `ExtAudioFile`, which cannot open MKV,
WebM or Opus. The spike (`docs/references/spike-native-recording.md` §4)
showed symphonia (all formats, plus libopus) and `AVAssetReader` together
read everything without FFmpeg, neither alone.

Watched folders must import each new file once it has finished being
written, survive restarts, never import a file twice and keep a visible
list of failures.

## Decision

1. **An import converts the file's audio into Deino's own format.**
   `deino-audio::media` streams the first audio track through a
   `MediaDecoder` (symphonia first, `AVAssetReader` on any failure) into
   `<recordings>/<meeting>/audio.m4a`, 48 kHz mono AAC, the same format as a
   live recording's mixed file. The meeting is created as `source =
   'import'` with that file as `audio_path`, then moved to `processing`; the
   pipeline does the rest exactly as for a record-only meeting. A video is
   copied beside it as `video.<ext>` (`video_path`) for the player.
2. **Watched folders poll** every 2 s. A file is ready once its size and
   modification time have not changed for 10 s (`StabilityTracker`, on the
   injected clock). Each handled file is a row in `watched_files`, keyed by
   folder, path, size and modification time: the de-duplication ledger and,
   for rows with a `failure`, the failure list (retry deletes the row,
   dismiss stamps it). Files already in a folder when it is added are entered
   as handled, so only new recordings import.

## Alternatives

- **Decode in the pipeline instead of converting** (give
  `OfflineTranscriber` the symphonia/AVFoundation chain and keep the
  original file as the recording). Saves the AAC encode (about 23 s for two
  hours), but every other reader of `audio_path` (diarization, clips, the
  player, exports) would need the same chain, and MKV/WebM do not play in
  WKWebView anyway.
- **`FSEvents` (`notify`) instead of polling.** Still needs a timer for
  the stability rule, reports unreliably on network shares and external
  drives, and adds a dependency; a few `readdir`s every 2 s cost nothing.
- **A content hash for de-duplication.** Catches a file copied back in with
  a new modification time, but reads the whole file (seconds for a long
  video) before anything else happens. Path, size and modification time is
  what the card allows and is free; a hash can be added to the ledger later.
- **Watched folders in `settings.json`.** The failure list and the ledger
  need queries and a foreign key to the folder, so both live in SQLite.

## Consequences

- Imports and live recordings look the same to every stage; `MeetingSource`
  tells them apart. FFmpeg now only serves the legacy re-transcription.
- An import costs one decode and encode (23 s for a two-hour MP4 at load
  ~55) before the meeting exists, and the disk holds the converted audio
  next to any kept video.
- A file copied into a folder again gets a new modification time and
  imports again; moving files aside ("Move to processed") avoids that.
- symphonia ignores AAC priming, so AAC sources start about 44 ms late;
  inaudible for an import.
