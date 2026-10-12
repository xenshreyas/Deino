# 0048. Clips cut by passthrough export, and comments anchored by line, offsets and quote

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** TRN-04 (Clips and moments), TRN-06 (Comments on transcript and
  notes); `docs/research/circleback.md` §7.3–§7.5, ADR 0033, ADR 0040

## Context

Circleback lets a user share a moment (a link to a time), cut a named clip
from a transcript range, and comment on highlighted transcript words with
threaded replies. Deino is single-user and local: a moment is a
`deino://` link (DEV-05 will open it), a clip is a file on the Mac, and a
comment is a private annotation. PARITY_PLAN D11 rules out FFmpeg for
cuts; the native-recording spike cut AAC sample-exactly with
`AVAssetExportSession` (Passthrough, `timeRange`) in 11 ms. A comment must
survive what happens to a transcript after it is written: search
re-indexing, edits of other lines (TRN-02), rows shifting as lines arrive,
an edit of its own line, and a re-transcription (CAP-08) that replaces
every line.

## Decision

**Moments.** `deino://meeting/<id>?t=<ms>`, the shape the `deino` CLI and
MCP tools already print and DEV-05's card names. EXP-01's exports still
write `deino://meetings/<id>`; DEV-05 registers one and accepts both
(PARITY_PLAN §7.2).

**Clips.**

- `deino_audio::clip::cut_clip`: `AVAssetExportSession`, Passthrough, a
  `timeRange` in a millisecond timescale. No re-encode, so the cut is fast
  and lossless; the edit list trims to the sample. Measured on the
  stand-up fixture: start and duration off by 0 ms (±250 ms allowed;
  `crates/deino-audio/tests/clip_timing.rs` decodes both files and finds
  the clip in the source by least squared error).
- The source is the screen recording when there is one (an `.mp4`; it
  keeps whatever tracks CAP-09 writes), else the mixed `audio.mp4` (an
  `.m4a`). `ClipSources` is implemented by `PlayerService`, so clips and
  the player agree on what exists and where.
- Files go in `<recording folder>/clips/`: inside the asset protocol's
  scope and the folder a meeting's purge removes. Names come from the
  user's name with separators, control and reserved characters removed,
  80 characters at most, and ` 2`, ` 3` … appended when the name is taken
  on disk or by a row (case-insensitively, as APFS compares). A rename
  moves the file to a free name. `path` is `UNIQUE`.
- Reveal goes through Rust only for a file inside the recordings folder
  (symlinks resolved); no command takes a path from the UI.
- Play uses the meeting's player (`playerActions.playRange`): the range,
  paused at its end, instead of a second player.

**Comments.**

- Tables `comment_threads` (meeting, target, `segment_id`, start and end
  offsets, the quote, `resolved_at`) and `comments` (the thread's
  comments). `segment_id` references `transcript_segments` with
  `ON DELETE SET NULL`: a re-transcription detaches a thread but never
  deletes it.
- A transcript thread is anchored in one line: its segment ID (stable
  across edits, re-indexing and re-saves), UTF-16 offsets into the line's
  text (what the webview counts) and the quoted words. A selection across
  lines keeps its first line's part. Notes threads anchor in the editor's
  whole text the same way.
- The UI finds the span again with `locateQuote`: at the offsets if the
  quote is still there, else at the occurrence nearest the old start, else
  nowhere: the thread is shown "Detached" in the Comments panel, kept.
- Highlights use the CSS Custom Highlight API (WebKit 17.2+, which
  macOS 14.2 ships): the transcript's word marks, search hits and the
  TipTap document are untouched, and the highlight is repainted on DOM
  mutations of the virtualised list or the editor.

## Alternatives

- **Cut with FFmpeg or re-encode with the AppleM4A preset.** FFmpeg is
  being removed (D11); re-encoding is slower and loses quality for nothing.
- **Clips in a global `<app data>/clips/` folder.** Outside the asset
  scope, and left behind when a meeting is purged.
- **Anchor comments by row index or by time.** Indexes shift when lines
  arrive or are deleted; times stay but the words move when a line is
  edited or split.
- **A ProseMirror mark or `<mark>` elements for highlights.** A mark would
  change the stored notes and every Markdown export; `<mark>` fights the
  transcript's own word and search spans.

## Consequences

- A clip of a meeting with a screen recording has the screen recording's
  tracks; if CAP-09 writes video without audio, clips need a composition
  with `audio.mp4` (follow-up for CAP-09).
- Notes threads are resolved in the UI only; Rust stores their anchors.
- Comments and clips of a trashed meeting are never listed (`deleted_at`),
  and go with the meeting when it is purged (`ON DELETE CASCADE`).
