# Retention runs with the trash purge, and stamps what it removed

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** PRV-01 (Retention policies)

## Context

PRV-01 adds three retention periods in Settings → Privacy & security
(recordings, transcripts, whole meetings: never, 7, 30, 90 or 365 days) and
a daily job that deletes what has expired. The card asks that:

- the transcript is kept at least as long as the audio;
- deleting a transcript keeps the notes, the meeting then says "Transcript
  deleted", and regenerating notes is disabled;
- "everything" reuses ORG-07's trash and its 30-day purge (ADR 0046);
- nothing outside the app's folders is ever deleted.

Three questions shape the design: where the job runs, how the app knows a
transcript was deleted rather than never made, and which files a recording
owns.

## Decision

**One housekeeping loop.** `TrashPurger` (ADR 0046) already runs at launch
and every 6 hours until exit, with a cancellation token stopped in
`lifecycle.rs`. It now takes extra `Housekeeping` jobs (`TrashPurger::with`)
that run before each purge, and `RetentionService` is one. "Daily" is an
upper bound: an item goes at most 6 hours after it expires. Running before
the purge means a meeting retention moves to the trash is counted from that
moment like any other. The retention service is built in
`composition/retention.rs` but called from `meetings::attach_trash`, where
the trash, its file adapter and the loop are built; an attach hook would
run after the loop is spawned.

**Rows first, then files, as in the trash.** Each sweep step is one
transaction over a JSON array of IDs:

| Period | Rows | Files |
|---|---|---|
| everything | `TrashService::trash` (cancels the running stage) | the purge, 30 days later |
| transcript | `DELETE transcript_segments` (FTS triggers drop the search documents), `DELETE embeddings WHERE ref_kind = 'transcript'` (they copy the text), stamp `transcript_deleted_at`; then `MeetingWork::reindex` so the in-memory semantic index forgets them | none |
| audio | null `audio_*`/`video_path` and the merge parts' paths, delete `clips` rows, stamp `audio_deleted_at` | `recordings/<id>` (clips live in it) and any recorded path inside `recordings/` |

A crash between the commit and the file deletion leaves orphaned files,
never a meeting pointing at missing audio. The order inside a sweep is
everything, transcript, audio.

**Two stamps on `meetings`.** `audio_deleted_at` and `transcript_deleted_at`
say what retention removed and when. The meeting page reads them through the
`retention` plugin's `status` command (`DbChanged` on `meetings` keeps it
fresh). `MeetingState.transcript_deleted_at` feeds the enhance material, so
`EnhanceService::enhance` refuses with `TranscriptDeleted` and the pipeline's
`enhance_notes` stage skips; the UI disables every regenerate trigger
through `useEnhanceAction().blocked`. Without the stamp, a meeting whose
transcript was deleted would look like one with only notes, and
regenerating would overwrite good notes with notes written from the user's
lines alone.

**The file-deletion path is the trash's.** `DiskMeetingFiles` gains
`delete_recordings`: the trash's `delete` is now `delete_recordings` plus
the `meetings/<id>` note-images folder. Both keep the rules of ADR 0046: a
path is deleted only when it is absolute, under `recordings/` and has no
`..` step; a folder is named only by a meeting ID of letters, digits and
dashes; `remove_dir_all` unlinks a symlink instead of following it. An
imported original outside the app's folders is never deleted; retention
forgets its path.

**What counts.** Only finished meetings (`ready`, `failed`) outside the
trash, aged from `created_at` (inclusive: a 7-day period deletes on day 7).
Scheduled, recording and processing meetings are never touched; trashed
ones are the purge's.

**The rule is in Rust and the UI.** `RetentionSettings::validate` rejects a
transcript period shorter than the audio one (`privacy.retention.transcript
must be kept at least as long as the audio`); the settings page disables
the shorter transcript choices, explains why, and raises the transcript
period when the user lengthens the audio one.

**The pause tip.** `privacy.pauseTipSeen` is a setting, so the record panel
(which reads it through a new `settings:default` grant on its capability)
shows "Pause for sensitive discussions" during one recording only: it is
marked seen on dismiss or when that recording ends.

## Alternatives

- **A separate daily scheduler for retention.** A second loop, token and
  `lifecycle.rs` stop for the same "count in days, run at launch" need.
  Rejected for the shared loop the card asks to reuse.
- **Derive "transcript deleted" from "no segments".** Indistinguishable from
  a meeting that never had a transcript (notes only, an import still
  waiting), where Enhance must keep working.
- **Trash the transcript instead of deleting it.** Retention exists to make
  data go; a second waiting period would surprise the user. The meeting-level
  "everything" already goes through the trash.
- **Keep the tip's state in `localStorage`.** Per-webview and outside the
  typed settings; the settings document is where the app keeps preferences.

## Consequences

- `meetings` has two more columns. New readers of a meeting's recording or
  transcript need not check them: the paths are NULL and the segments gone.
- The legacy `meetings.folder_path` still names the deleted recordings
  folder; the inherited "open recording folder" button opens nothing.
- Retention runs up to four times a day; each run is a few indexed queries
  when nothing has expired.
