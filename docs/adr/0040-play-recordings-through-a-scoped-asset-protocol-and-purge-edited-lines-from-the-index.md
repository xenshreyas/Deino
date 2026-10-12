# 0040. Play recordings through a scoped asset protocol, and purge edited lines from the index in the edit's transaction

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** TRN-03 (synced audio and video player), TRN-02 (transcript
  editing, deletion, search and replace); `docs/research/circleback.md` §7.1,
  §7.6, `docs/research/granola.md` §4, ADR 0026, ADR 0030, ADR 0032

## Context

TRN-03 plays a meeting's recording (ADR 0026: `audio.mp4` mixed, `mic.m4a`
"Me", `system.m4a` "Them", a CAP-09 screen recording later) beside its
transcript, with seeks accurate to ±200 ms, and lets the user download each
file. The app granted the webview `fs:read-all`, `fs:write-all` and an
`$APPDATA/**` asset scope, although the fs plugin was never initialised and
no code read files through it: the grant was dead but would have let any
script in the webview read every file once someone wired the plugin.

TRN-02 lets the user edit and delete transcript lines, and find and replace
across meetings. "Deleted text never appears in search" has two indexes to
honour (ADR 0032): `search_documents`/FTS5, which triggers already keep in
step with `text` and `is_deleted`, and the semantic `embeddings`, whose rows
copy the text of ~400-token transcript windows and are written only by the
`index` stage and the backfill. A window covering a deleted line keeps the
deleted words until the meeting is embedded again, which may be never if the
model is gone or the backfill does not run; and re-embedding needs the local
model, so it cannot sit inside the user's edit.

## Decision

**Playback.**

- The webview loads recordings with `convertFileSrc` through Tauri's asset
  protocol, whose scope is now `$APPDATA/recordings/**` (and the notepad's
  images, `$APPDATA/meetings/**`). Every `fs:*` permission, the fs scope and
  the `tauri-plugin-fs` / `@tauri-apps/plugin-fs` dependencies are removed:
  nothing used them.
- Rust decides what can be played. `plugin:player|media` lists the files a
  meeting's row names that exist inside the recordings folder (symlinks
  resolved, `FsMediaFiles`), so the UI never asks for a path the scope would
  refuse. `plugin:player|download` takes a `MediaSource`, not a path, and
  opens the save dialog in Rust (the export's `SaveDialog` port).
- One `PlaybackController` (the UI's one class: it models a real object with
  behaviour) owns a hidden `<audio>` element and its source. It keeps a seek
  asked for before the metadata loads (a `?t=` link, "Play from here" on a
  fresh page) and applies it the moment it can, so a seek lands where it was
  asked to the element's own precision; switching Mixed/Me/Them keeps the
  position, speed and play state; a screen recording follows the audio and
  is pulled back past 250 ms of drift. While playing it reports the position
  every animation frame into a Zustand store; the transcript selects only the
  playing row (and, inside that one row, the word), so the list re-renders a
  few times a minute, not 60 times a second.

**Edits.**

- `transcript_segments.original_text` keeps the engine's text from the first
  edit; edits stamp `edited_at` (which NOT-02's `EnhanceBasis` already turns
  into the "transcript edited" regenerate reason).
- A trigger (`embeddings_segments_after_edit`) deletes every transcript
  window overlapping a line the user edits (a new `edited_at`) or
  soft-deletes, in the same transaction as the edit. The in-memory vector
  index keeps those vectors until the next re-index, but search hydrates
  hits from `embeddings`, so a dropped row is never shown. Diarization's text
  rewrites (no `edited_at`) leave windows alone.
- `search::ReindexQueue` re-embeds the touched meetings afterwards: one
  worker on a tracked task, meetings coalesced, in order, so an older read of
  the transcript can never land after a newer one and bring words back.
- Deleting through the legacy `transcripts` view (re-transcription) now also
  removes the meeting's soft-deleted rows, so an undo cannot resurrect lines
  of the old transcription.

## Alternatives

- **Keep `fs:read-all` and read files with the fs plugin's `readFile` into a
  Blob URL.** Loads whole recordings into memory, cannot stream or seek
  before the download finishes, and keeps a broad grant.
- **A custom `deino-media://` protocol in Rust.** More code (range requests,
  MIME types) for what the asset protocol already does, scoped.
- **A dynamic asset scope (`allow_file`) per meeting.** Only useful for
  recordings outside the recordings folder, and no such recording exists:
  imports do not set audio paths today (CAP-11's follow-up below).
- **Re-embed inside the edit command.** Blocks every edit on the local
  embedding model (a cold load takes seconds) and fails the edit when the
  model is missing; the trigger makes the purge independent of the model.
- **Rerun the `index` pipeline stage after an edit.** Moves a `ready` meeting
  back to `processing` (the header's strip lights up) for a background
  chore; the queue is invisible.
- **Drop the meeting's whole transcript embedding on any edit.** Simpler SQL,
  but one typo fix would make the whole meeting unsearchable by meaning until
  the re-index.

## Consequences

- No webview code can read arbitrary files; a recording outside
  `<app data>/recordings` is not playable until something copies it there.
- Seek accuracy is measured in `e2e/player.spec.ts` over the standup
  fixture's audio: 19 seeks (line starts, word starts, and seeks during
  playback read at `seeked`), worst error 6–11 ms against ±200 ms.
- A narrow race remains: a backfill or `index` stage run that read the
  transcript before an edit and writes its chunks after the queue's re-index
  could put the old window back until that meeting is indexed again.
  Closing it needs `SqliteEmbeddingStore::replace` to compare the
  transcript's basis (newest `edited_at`) inside its transaction (a §7.2
  follow-up).
- The `deino` CLI and MCP server (ADR 0035) read through the same
  repositories and indexes, so they inherit both rules without code.
